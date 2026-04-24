/// Windows sleep inhibitor — prevents the OS from sleeping during active
/// inference or tool execution. Uses `SetThreadExecutionState` with
/// `ES_SYSTEM_REQUIRED` so the system stays awake without forcing the display on.
///
/// On non-Windows platforms this is a zero-cost no-op struct.
///
/// Usage: create a `SleepInhibitor` at the start of a model turn; it
/// automatically restores normal power policy when it drops.

#[cfg(target_os = "windows")]
mod imp {
    const ES_CONTINUOUS: u32 = 0x80000000;
    const ES_SYSTEM_REQUIRED: u32 = 0x00000001;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(esFlags: u32) -> u32;
    }

    pub struct SleepInhibitor;

    impl SleepInhibitor {
        pub fn acquire() -> Self {
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
            }
            Self
        }
    }

    impl Drop for SleepInhibitor {
        fn drop(&mut self) {
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub struct SleepInhibitor;
    impl SleepInhibitor {
        pub fn acquire() -> Self {
            Self
        }
    }
}

pub use imp::SleepInhibitor;
