mod apps_config;
pub mod performance;
mod settings;
mod toolbar_items;
mod weg_items;

pub use toolbar_items::TOOLBAR_ITEMS_MANAGER;
pub use weg_items::WEG_ITEMS_MANAGER;

use arc_swap::ArcSwap;
use notify_debouncer_full::{
    new_debouncer,
    notify::{ReadDirectoryChangesWatcher, RecursiveMode, Watcher},
    DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use seelen_core::{
    resource::ResourceKind,
    state::{AppsConfigurationList, CssStyles, Settings, SluPopupConfig, SluPopupContent},
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

use crate::{
    error::{Result, ResultLogExt},
    log_error,
    resources::RESOURCES,
    utils::constants::SEELEN_COMMON,
    widgets::popups::POPUPS_MANAGER,
};

pub static FULL_STATE: LazyLock<Arc<ArcSwap<FullState>>> = LazyLock::new(|| {
    Arc::new(ArcSwap::from_pointee({
        log::trace!("Creating new State Manager");
        FullState::new().expect("Failed to create State Manager")
    }))
});

#[derive(Debug, Clone)]
pub struct FullState {
    watcher: Arc<Option<Debouncer<ReadDirectoryChangesWatcher, FileIdMap>>>,
    // ======== data ========
    pub settings: Settings,
    pub settings_by_app: AppsConfigurationList,
}

unsafe impl Sync for FullState {}

impl FullState {
    fn new() -> Result<Self> {
        let mut manager = Self {
            watcher: Arc::new(None),
            // ======== data ========
            settings: Settings::default(),
            settings_by_app: AppsConfigurationList::default(),
        };
        manager.load_all();
        manager.start_listeners()?;
        Ok(manager)
    }

    /// Shorthand of `FullState::clone` on Arc reference
    ///
    /// Intended to be used with `ArcSwap::rcu` to mofify the state
    pub fn cloned(&self) -> Self {
        self.clone()
    }

    fn join_and_filter_debounced_changes(events: Vec<DebouncedEvent>) -> HashSet<PathBuf> {
        let mut result = HashSet::new();
        for event in events {
            for path in event.event.paths {
                if !path.is_dir() {
                    result.insert(path);
                }
            }
        }
        result
    }

    fn process_changes(&mut self, changed: &HashSet<PathBuf>) -> Result<()> {
        let mut widgets_changed = false;
        let mut icons_changed = false;
        let mut themes_changed = false;
        let mut plugins_changed = false;
        let mut wallpapers_changed = false;

        let mut settings_changed = false;

        // Single iteration over the changed paths
        for path in changed {
            if !icons_changed && path.starts_with(SEELEN_COMMON.user_icons_path()) {
                icons_changed = true;
            };

            if !themes_changed
                && (path.starts_with(SEELEN_COMMON.user_themes_path())
                    || path.starts_with(SEELEN_COMMON.bundled_themes_path()))
            {
                themes_changed = true;
            }

            if !plugins_changed
                && (path.starts_with(SEELEN_COMMON.user_plugins_path())
                    || path.starts_with(SEELEN_COMMON.bundled_plugins_path()))
            {
                plugins_changed = true;
            }

            if !widgets_changed
                && (path.starts_with(SEELEN_COMMON.user_widgets_path())
                    || path.starts_with(SEELEN_COMMON.bundled_widgets_path()))
            {
                widgets_changed = true;
            }

            if !settings_changed && path == SEELEN_COMMON.settings_path() {
                settings_changed = true;
            }

            if !wallpapers_changed && path.starts_with(SEELEN_COMMON.user_wallpapers_path()) {
                wallpapers_changed = true;
            }
        }

        if widgets_changed {
            log::info!("Widgets changed");
            RESOURCES.load_all_of_type(ResourceKind::Widget)?;
            RESOURCES.emit_widgets()?;
        }

        if themes_changed {
            log::info!("Themes changed");
            RESOURCES.load_all_of_type(ResourceKind::Theme)?;
            RESOURCES.emit_themes();
        }

        if plugins_changed {
            log::info!("Plugins changed");
            RESOURCES.load_all_of_type(ResourceKind::Plugin)?;
            RESOURCES.emit_plugins();
        }

        if wallpapers_changed {
            log::info!("Wallpapers changed");
            RESOURCES.load_all_of_type(ResourceKind::Wallpaper)?;
            RESOURCES.emit_wallpapers();

            if self.sanitize_wallpaper_collections() {
                self.emit_settings()?;
            }
        }

        if icons_changed {
            log::info!("Icon Packs changed");
            RESOURCES.load_all_of_type(ResourceKind::IconPack)?;
            RESOURCES.emit_icon_packs();
        }

        // important: settings changed should be the last one to avoid use unexisting state
        // like new recently added theme, plugin, widget, etc
        if settings_changed {
            log::info!("Seelen Settings changed");
            self.read_settings();
            self.emit_settings()?;
        }

        Ok(())
    }

    fn start_listeners(&mut self) -> Result<()> {
        log::trace!("Starting Seelen UI Files Watcher");
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            None,
            |result: DebounceEventResult| match result {
                Ok(events) => {
                    // log::info!("Seelen UI File Watcher events: {:?}", events);
                    let changed = Self::join_and_filter_debounced_changes(events);
                    FULL_STATE.rcu(move |state| {
                        let mut state = state.cloned();
                        log_error!(state.process_changes(&changed));
                        state
                    });
                }
                Err(errors) => errors
                    .iter()
                    .for_each(|e| log::error!("File Watcher Error: {e:?}")),
            },
        )?;

        debouncer
            .watcher()
            .watch(SEELEN_COMMON.app_data_dir(), RecursiveMode::Recursive)?;
        self.watcher = Arc::new(Some(debouncer));
        Ok(())
    }

    fn load_all(&mut self) {
        let settings_path = SEELEN_COMMON.settings_path();

        let (settings_res, bundled_res) = std::thread::scope(|s| {
            let settings = s.spawn(|| {
                settings_path
                    .exists()
                    .then(|| Settings::load(settings_path))
            });
            let bundled = s.spawn(Self::load_bundled_settings_by_app);
            (settings.join(), bundled.join())
        });

        if let Ok(Some(res)) = settings_res {
            match res {
                Ok(settings) => {
                    self.settings = settings;
                    self.migration_v2_5_0().log_error();
                    self.sanitize_wallpaper_collections();
                }
                Err(e) => {
                    log::error!("Failed to read settings: {e}");
                    Self::show_corrupted_state_to_user(SEELEN_COMMON.settings_path());
                }
            }
        }

        if let Ok(res) = bundled_res {
            match res {
                Ok(apps) => self.settings_by_app = apps,
                Err(e) => log::error!("Error loading bundled app configs: {e}"),
            }
        }
    }

    fn show_corrupted_state_to_user(path: &Path) {
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let mut manager = POPUPS_MANAGER.lock();
            let config = SluPopupConfig {
                title: vec![SluPopupContent::Group {
                    items: vec![
                        SluPopupContent::Icon {
                            name: "BiSolidError".to_string(),
                            styles: Some(
                                CssStyles::new()
                                    .add("color", "var(--color-red-800)")
                                    .add("height", "1.2rem"),
                            ),
                        },
                        SluPopupContent::Text {
                            value: t!("runtime.corrupted_data").to_string(),
                            styles: None,
                        },
                    ],
                    styles: Some(CssStyles::new().add("alignItems", "center")),
                }],
                content: vec![
                    SluPopupContent::Text {
                        value: t!("runtime.corrupted_file").to_string(),
                        styles: None,
                    },
                    SluPopupContent::Text {
                        value: format!("{}: {:?}", t!("runtime.corrupted_file_path"), path),
                        styles: None,
                    },
                ],
                ..Default::default()
            };
            log_error!(manager.create(config));
        });
    }
}
