use std::{
    fs::File,
    io::{self, Read as _},
    path::Path,
};

use eyre::{Context as _, Report};
use pelite::pe::{Pe as _, PeFile, Rva, exports::By};
use yoke::{Yoke, Yokeable};

use crate::blob::Blob;

#[derive(Debug, thiserror::Error)]
pub enum DllError {
    #[error("{0}")]
    Report(#[from] Report),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Pelite(#[from] pelite::Error),
}

pub struct Dll(Yoke<DllRef<'static>, Blob>);

impl Dll {
    pub fn new<P: AsRef<Path>>(dll: P) -> Result<Self, DllError> {
        let mut file = File::open(dll)?;
        let size = file.metadata()?.len() as usize;

        let mut blob = Blob::new_zeroed(size)?;

        file.read_exact(&mut blob)?;

        let yoke = Yoke::try_attach_to_cart(blob, |data| {
            let file = PeFile::from_bytes(data).context("failed to parse file")?;
            let by = file.exports()?.by()?;

            let dll_file = DllRef { file, exports: by };

            Ok::<_, DllError>(dll_file)
        })?;

        Ok(Self(yoke))
    }

    /// Checks if symbol exists
    pub fn symbol_exists(&self, name: &str) -> bool {
        let file = self.0.get();
        file.exports.name(name).is_ok()
    }

    /// Gets rva for symbol
    pub fn symbol_rva(&self, name: &str) -> Option<Rva> {
        let file = self.0.get();
        let rva = file.exports.name(name).ok().and_then(|e| e.symbol())?;

        Some(rva)
    }

    /// Grab a reference to the inner DllRef
    pub fn object(&self) -> DllRef<'_> {
        *self.0.get()
    }

    /// Get the backing dll memory
    pub fn mem(&self) -> &[u8] {
        self.0.backing_cart()
    }
}

#[derive(Copy, Clone, Yokeable)]
pub struct DllRef<'a> {
    pub file: PeFile<'a>,
    pub exports: By<'a, PeFile<'a>>,
}
