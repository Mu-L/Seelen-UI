use std::{ffi::c_void, mem::zeroed, path::PathBuf, time::Instant};

use seelen_core::system_state::{MediaPlayerOwner, MediaPlayerTimeline};
use serde::Serialize;

use windows::{
    core::{Interface, Param, Result, GUID, HRESULT, PCWSTR},
    Devices::Custom::DeviceSharingMode,
    Win32::{
        Foundation::{BOOL, PROPERTYKEY},
        Media::Audio::{
            AudioObjectType, AudioObjectType_BackCenter, AudioObjectType_BackLeft,
            AudioObjectType_BackRight, AudioObjectType_BottomBackLeft,
            AudioObjectType_BottomBackRight, AudioObjectType_BottomFrontLeft,
            AudioObjectType_BottomFrontRight, AudioObjectType_FrontCenter,
            AudioObjectType_FrontLeft, AudioObjectType_FrontRight, AudioObjectType_LowFrequency,
            AudioObjectType_SideLeft, AudioObjectType_SideRight, AudioObjectType_TopBackLeft,
            AudioObjectType_TopBackRight, AudioObjectType_TopFrontLeft,
            AudioObjectType_TopFrontRight, ERole,
            Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback},
            IAudioSessionControl2, IAudioSessionEvents, IAudioSessionManager2,
            IAudioSessionNotification, WAVEFORMATEX,
        },
        System::Com::StructuredStorage::PROPVARIANT,
    },
};

#[allow(dead_code)]
pub struct ChannelMask {}

#[allow(non_upper_case_globals)]
#[allow(dead_code)]
impl ChannelMask {
    pub const Mono: AudioObjectType = AudioObjectType_FrontCenter;
    pub const Stereo: AudioObjectType =
        AudioObjectType(AudioObjectType_FrontLeft.0 | AudioObjectType_FrontRight.0);

    pub const Spatial_2_1: AudioObjectType =
        AudioObjectType(Self::Stereo.0 | AudioObjectType_LowFrequency.0);
    pub const Quad: AudioObjectType =
        AudioObjectType(Self::Stereo.0 | AudioObjectType_BackLeft.0 | AudioObjectType_BackRight.0);
    pub const Spatial_4_1: AudioObjectType =
        AudioObjectType(Self::Quad.0 | AudioObjectType_LowFrequency.0);
    pub const Spatial_5_1: AudioObjectType = AudioObjectType(
        Self::Stereo.0
            | AudioObjectType_FrontCenter.0
            | AudioObjectType_LowFrequency.0
            | AudioObjectType_SideLeft.0
            | AudioObjectType_SideRight.0,
    );

    pub const Spatial_7_1: AudioObjectType = AudioObjectType(
        Self::Spatial_5_1.0 | AudioObjectType_BackLeft.0 | AudioObjectType_BackRight.0,
    );

    pub const MaxStaticObjectCount_7_1_4: u32 = 12;
    pub const Spatial_7_1_4: AudioObjectType = AudioObjectType(
        Self::Spatial_7_1.0
            | AudioObjectType_TopFrontLeft.0
            | AudioObjectType_TopFrontRight.0
            | AudioObjectType_TopBackLeft.0
            | AudioObjectType_TopBackRight.0,
    );

    pub const MaxStaticObjectCount_7_1_4_4: u32 = 16;
    pub const Spatial_7_1_4_4: AudioObjectType = AudioObjectType(
        Self::Spatial_7_1_4.0
            | AudioObjectType_BottomFrontLeft.0
            | AudioObjectType_BottomFrontRight.0
            | AudioObjectType_BottomBackLeft.0
            | AudioObjectType_BottomBackRight.0,
    );

    pub const MaxStaticObjectCount_8_1_4_4: u32 = 17;
    pub const Spatial_8_1_4_4: AudioObjectType =
        AudioObjectType(Self::Spatial_7_1_4_4.0 | AudioObjectType_BackCenter.0);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlayer {
    pub umid: String,
    pub title: String,
    pub author: String,
    pub thumbnail: Option<PathBuf>,
    pub owner: MediaPlayerOwner,
    pub timeline: MediaPlayerTimeline,
    pub playing: bool,
    pub default: bool,
    #[serde(skip)]
    pub removed_at: Option<Instant>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDeviceSession {
    #[serde(skip)]
    pub controls: IAudioSessionControl2,
    #[serde(skip)]
    pub events_callback: IAudioSessionEvents,
    // ---
    pub id: String,
    pub instance_id: String,
    pub process_id: u32,
    pub name: String,
    pub icon_path: Option<PathBuf>,
    pub is_system: bool,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaDeviceType {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDevice {
    #[serde(skip)]
    pub volume_endpoint: IAudioEndpointVolume,
    #[serde(skip)]
    pub volume_callback: IAudioEndpointVolumeCallback,
    #[serde(skip)]
    pub session_manager: IAudioSessionManager2,
    #[serde(skip)]
    pub session_created_callback: IAudioSessionNotification,
    // ---
    pub id: String,
    pub name: String,
    pub r#type: MediaDeviceType,
    pub is_default_multimedia: bool,
    pub is_default_communications: bool,
    pub sessions: Vec<MediaDeviceSession>,
    pub volume: f32,
    pub muted: bool,
}

/* Windows IPolicyConfig UNDOCUMENTED INTERFACE */
#[allow(non_upper_case_globals)]
pub const PolicyConfig: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

windows_core::imp::define_interface!(
    IPolicyConfig,
    IPolicyConfig_Vtbl,
    0xf8679f50_850a_41cf_9c72_430f290290c8
);

windows_core::imp::interface_hierarchy!(IPolicyConfig, windows_core::IUnknown);
#[allow(non_snake_case)]
impl IPolicyConfig {
    pub unsafe fn GetMixFormat(&self, device_id: impl Param<PCWSTR>) -> Result<*mut WAVEFORMATEX> {
        let mut result__ = zeroed::<*mut WAVEFORMATEX>();
        (Interface::vtable(self).GetMixFormat)(
            Interface::as_raw(self),
            device_id.param().abi(),
            &mut result__,
        )
        .and_then(|| windows_core::Type::from_abi(result__))
    }

    pub unsafe fn GetDeviceFormat(
        &self,
        device_id: impl Param<PCWSTR>,
        default: impl Into<BOOL>,
    ) -> Result<*mut WAVEFORMATEX> {
        let mut result__ = zeroed::<*mut WAVEFORMATEX>();
        (Interface::vtable(self).GetDeviceFormat)(
            Interface::as_raw(self),
            device_id.param().abi(),
            default.into().0,
            &mut result__,
        )
        .and_then(|| windows_core::Type::from_abi(result__))
    }

    pub unsafe fn ResetDeviceFormat(&self, device_id: impl Param<PCWSTR>) -> Result<()> {
        (Interface::vtable(self).ResetDeviceFormat)(
            Interface::as_raw(self),
            device_id.param().abi(),
        )
        .ok()
    }

    pub unsafe fn SetDeviceFormat(
        &self,
        device_id: impl Param<PCWSTR>,
        mut endpoint_format: WAVEFORMATEX,
        mut mix_format: WAVEFORMATEX,
    ) -> Result<()> {
        (Interface::vtable(self).SetDeviceFormat)(
            Interface::as_raw(self),
            device_id.param().abi(),
            &mut endpoint_format,
            &mut mix_format,
        )
        .ok()
    }

    pub unsafe fn GetProcessingPeriod(
        &self,
        device_id: impl Param<PCWSTR>,
        default: impl Into<BOOL>,
        default_period: *mut i64,
        min_period: *mut i64,
    ) -> Result<()> {
        (Interface::vtable(self).GetProcessingPeriod)(
            Interface::as_raw(self),
            device_id.param().abi(),
            default.into().0,
            default_period,
            min_period,
        )
        .ok()
    }

    pub unsafe fn SetProcessingPeriod(
        &self,
        device_id: impl Param<PCWSTR>,
        period: *mut i64,
    ) -> Result<()> {
        (Interface::vtable(self).SetProcessingPeriod)(
            Interface::as_raw(self),
            device_id.param().abi(),
            period,
        )
        .ok()
    }

    pub unsafe fn GetShareMode(&self, device_id: impl Param<PCWSTR>) -> Result<DeviceSharingMode> {
        let mut result__ = zeroed::<DeviceSharingMode>();
        (Interface::vtable(self).GetShareMode)(
            Interface::as_raw(self),
            device_id.param().abi(),
            &mut result__,
        )
        .and_then(|| windows_core::Type::from_abi(result__))
    }

    pub unsafe fn SetShareMode(
        &self,
        device_id: impl Param<PCWSTR>,
        mut mode: DeviceSharingMode,
    ) -> Result<()> {
        (Interface::vtable(self).SetShareMode)(
            Interface::as_raw(self),
            device_id.param().abi(),
            &mut mode,
        )
        .ok()
    }

    pub unsafe fn GetPropertyValue(
        &self,
        device_id: impl Param<PCWSTR>,
        bFxStore: impl Into<BOOL>,
        key: *const PROPERTYKEY,
    ) -> Result<PROPVARIANT> {
        let mut result__ = zeroed();
        (Interface::vtable(self).GetPropertyValue)(
            Interface::as_raw(self),
            device_id.param().abi(),
            bFxStore.into().0,
            key,
            &mut result__,
        )
        .map(|| result__)
    }

    pub unsafe fn SetPropertyValue(
        &self,
        device_id: impl Param<PCWSTR>,
        bFxStore: impl Into<BOOL>,
        key: *const PROPERTYKEY,
        propvar: *const PROPVARIANT,
    ) -> Result<()> {
        (Interface::vtable(self).SetPropertyValue)(
            Interface::as_raw(self),
            device_id.param().abi(),
            bFxStore.into().0,
            key,
            propvar,
        )
        .ok()
    }

    pub unsafe fn SetDefaultEndpoint(
        &self,
        device_id: impl Param<PCWSTR>,
        role: ERole,
    ) -> Result<()> {
        (Interface::vtable(self).SetDefaultEndpoint)(
            Interface::as_raw(self),
            device_id.param().abi(),
            role,
        )
        .ok()
    }

    pub unsafe fn SetEndpointVisibility(
        &self,
        device_id: impl Param<PCWSTR>,
        visible: impl Into<BOOL>,
    ) -> Result<()> {
        (Interface::vtable(self).SetEndpointVisibility)(
            Interface::as_raw(self),
            device_id.param().abi(),
            visible.into().0,
        )
        .ok()
    }
}

#[repr(C)]
#[doc(hidden)]
#[allow(non_snake_case, non_camel_case_types)]
pub struct IPolicyConfig_Vtbl {
    pub base__: ::windows::core::IUnknown_Vtbl,
    pub GetMixFormat:
        unsafe extern "system" fn(this: *mut c_void, PCWSTR, *mut *mut WAVEFORMATEX) -> HRESULT,
    pub GetDeviceFormat: unsafe extern "system" fn(
        this: *mut c_void,
        PCWSTR,
        i32,
        *mut *mut WAVEFORMATEX,
    ) -> HRESULT,
    pub ResetDeviceFormat: unsafe extern "system" fn(this: *mut c_void, PCWSTR) -> HRESULT,
    pub SetDeviceFormat: unsafe extern "system" fn(
        this: *mut c_void,
        PCWSTR,
        *mut WAVEFORMATEX,
        *mut WAVEFORMATEX,
    ) -> HRESULT,
    pub GetProcessingPeriod:
        unsafe extern "system" fn(this: *mut c_void, PCWSTR, i32, *mut i64, *mut i64) -> HRESULT,
    pub SetProcessingPeriod:
        unsafe extern "system" fn(this: *mut c_void, PCWSTR, *mut i64) -> HRESULT,
    pub GetShareMode:
        unsafe extern "system" fn(this: *mut c_void, PCWSTR, *mut DeviceSharingMode) -> HRESULT,
    pub SetShareMode:
        unsafe extern "system" fn(this: *mut c_void, PCWSTR, *mut DeviceSharingMode) -> HRESULT,
    pub GetPropertyValue: unsafe extern "system" fn(
        this: *mut c_void,
        PCWSTR,
        i32,
        *const PROPERTYKEY,
        *mut PROPVARIANT,
    ) -> HRESULT,
    pub SetPropertyValue: unsafe extern "system" fn(
        this: *mut c_void,
        PCWSTR,
        i32,
        *const PROPERTYKEY,
        *const PROPVARIANT,
    ) -> HRESULT,
    pub SetDefaultEndpoint: unsafe extern "system" fn(this: *mut c_void, PCWSTR, ERole) -> HRESULT,
    pub SetEndpointVisibility: unsafe extern "system" fn(this: *mut c_void, PCWSTR, i32) -> HRESULT,
}
