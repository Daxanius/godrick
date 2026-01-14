use libloading::{Library, Symbol};
use std::{path::PathBuf, str::FromStr, sync::Arc};

use crate::{Error, Result};

// Define file extensions based on target OS
#[cfg(target_os = "windows")]
pub const EXTENSION: &str = "dll";

#[cfg(target_os = "linux")]
pub const EXTENSION: &str = "so";

#[cfg(target_os = "macos")]
pub const EXTENSION: &str = "dylib";

pub type LimbInitFn = unsafe extern "C" fn();
pub type LimbTickFn = unsafe extern "C" fn(memory: *mut u8, size: usize, ptr: usize);

pub struct Limb {
    pub spec: LimbSpec,
    _lib: Arc<Library>,
    f_init: Symbol<'static, LimbInitFn>,
    f_tick: Symbol<'static, LimbTickFn>,
}

impl Limb {
    pub fn load(spec: &LimbSpec) -> Result<Self> {
        // Load library into an Arc to share + own
        let lib = Arc::new(unsafe {
            Library::new(&spec.path())
                .map_err(|e| Error::Limb(format!("Failed to load limb: {e}")))?
        });

        // SAFETY: This is fine — we leak the Arc temporarily to extend symbol lifetimes
        let raw = Arc::into_raw(Arc::clone(&lib));

        let f_init: Symbol<LimbInitFn> = unsafe {
            (*raw)
                .get(b"init\0")
                .map_err(|e| Error::Limb(format!("Missing symbol 'init': {e}")))?
        };

        let f_tick: Symbol<LimbTickFn> = unsafe {
            (*raw)
                .get(b"run\0")
                .map_err(|e| Error::Limb(format!("Missing symbol 'run': {e}")))?
        };

        // Immediately reconstruct Arc to avoid memory leak
        unsafe {
            let _ = Arc::from_raw(raw);
        }

        // Transmute lifetimes of symbol to 'static (safe here because lib is held in Arc)
        let f_init = unsafe {
            std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn()>,
                libloading::Symbol<'_, unsafe extern "C" fn()>,
            >(f_init)
        };

        let f_tick = unsafe {
            std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut u8, usize, usize)>,
                libloading::Symbol<'_, unsafe extern "C" fn(*mut u8, usize, usize)>,
            >(f_tick)
        };

        Ok(Self {
            spec: spec.clone(),
            _lib: lib,
            f_init,
            f_tick,
        })
    }

    #[inline]
    pub fn init(&self) {
        unsafe { (self.f_init)() }
    }

    #[inline]
    pub fn tick(&self, memory: &mut [u8], ptr: usize) {
        unsafe { (self.f_tick)(memory.as_mut_ptr(), memory.len(), ptr) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimbSpec {
    path: PathBuf,
    offset: Option<u64>,
}

impl LimbSpec {
    #[must_use]
    pub fn new(path: PathBuf, offset: Option<u64>) -> Self {
        LimbSpec { path, offset }
    }

    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    #[must_use]
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }
}

impl FromStr for LimbSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        parse_limb(s)
    }
}

impl TryFrom<&str> for LimbSpec {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        parse_limb(s)
    }
}

impl TryFrom<String> for LimbSpec {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        parse_limb(&s)
    }
}

fn parse_limb(s: &str) -> Result<LimbSpec> {
    let mut offset: Option<u64> = None;
    let mut path;

    match s.split_once('@') {
        Some((path_str, offset_str)) => {
            path = PathBuf::from(path_str);
            offset = Some(
                u64::from_str_radix(offset_str.trim_start_matches("0x"), 16)
                    .map_err(|_| Error::Limb("Invalid offset in: {s}".to_string()))?,
            );
        }
        None => {
            path = PathBuf::from(s);
        }
    }

    if path.extension().is_none() {
        path.set_extension(EXTENSION);
    }

    Ok(LimbSpec::new(path, offset))
}
