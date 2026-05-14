pub mod cli;
pub mod handlers;

use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    Graphics::Gdi::{InvalidateRect, UpdateWindow},
    UI::WindowsAndMessaging::{
        FindWindowA, FindWindowExA, GetParent, PostMessageW, SetParent, SetWindowPos, HWND_BOTTOM,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
    },
};

use crate::{
    error::Result,
    pcstr,
    windows_api::{WindowEnumerator, WindowsApi},
};

pub struct SeelenWall {}

impl SeelenWall {
    fn find_worker_w() -> Result<HWND> {
        let progman = unsafe { FindWindowA(pcstr!("Progman"), None)? };

        // Send 0x052C to Progman to ensure WorkerW is spawned.
        unsafe { PostMessageW(Some(progman), 0x052C, WPARAM(0xD), LPARAM(0x1))? };

        // CASE 1: Standard Windows 10/11 layout
        // WorkerW exists as a sibling to another WorkerW that contains SHELLDLL_DefView
        // 0x00010190 "" WorkerW
        //   0x000100EE "" SHELLDLL_DefView
        //     0x000100F0 "FolderView" SysListView32
        // 0x00100B8A "" WorkerW       <-- This is the WorkerW we want
        // 0x000100EC "Program Manager" Progman
        // We enumerate all Windows, until we find one, that has the SHELLDLL_DefView as a child.
        // If we found that window, we take its next sibling and assign it to workerw.
        let mut worker_w = None;
        WindowEnumerator::new().for_each(|current| unsafe {
            if FindWindowExA(Some(current.hwnd()), None, pcstr!("SHELLDLL_DefView"), None).is_ok() {
                if let Ok(w) = FindWindowExA(None, Some(current.hwnd()), pcstr!("WorkerW"), None) {
                    worker_w = Some(w);
                }
            }
        })?;

        // CASE 2: Raised Desktop (Windows 11 with layered shell view)
        // Progman contains SHELLDLL_DefView and WorkerW as children
        // 0x000100EC "Program Manager" Progman
        //   0x000100EE "" SHELLDLL_DefView
        //     0x000100F0 "FolderView" SysListView32
        //   0x00100B8A "" WorkerW       <-- This is the WorkerW we want
        if worker_w.is_none() {
            let mut attempts = 0;
            worker_w = unsafe { FindWindowExA(Some(progman), None, pcstr!("WorkerW"), None).ok() };
            while worker_w.is_none() && attempts < 10 {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
                worker_w =
                    unsafe { FindWindowExA(Some(progman), None, pcstr!("WorkerW"), None).ok() };
            }
        }

        worker_w.ok_or("WorkerW not found".into())
    }

    fn try_set_under_desktop_items(hwnd: HWND) -> Result<()> {
        let worker_w = Self::find_worker_w()?;

        unsafe {
            if GetParent(hwnd).ok() != Some(worker_w) {
                SetParent(hwnd, Some(worker_w))?;
            }
            SetWindowPos(
                hwnd,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER,
            )?;
        }

        Ok(())
    }

    /// this is only needed on the case 2 of try_set_inside_workerw
    fn refresh_desktop() -> Result<()> {
        unsafe {
            let progman = FindWindowA(pcstr!("Progman"), None)?;
            if let Ok(shell_view) =
                FindWindowExA(Some(progman), None, pcstr!("SHELLDLL_DefView"), None)
            {
                InvalidateRect(Some(shell_view), None, true).ok()?;
                UpdateWindow(shell_view).ok()?;
            }
        }
        WindowsApi::refresh_desktop()
    }
}
