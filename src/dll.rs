use std::{fs::File, io::Read as _, ops::Deref, path::Path};

use eyre::Context as _;
use pelite::pe::{Pe as _, PeFile, Rva, exports::By};
use stable_deref_trait::StableDeref;
use yoke::{Yoke, Yokeable};

use crate::{PluginError, blob::Blob};

type Inner = Yoke<DllRef<'static>, Blob>;

pub struct Dll(Inner);

// Safety: Blob is StableDeref
unsafe impl StableDeref for Dll {}

impl Dll {
    pub fn new<P: AsRef<Path>>(dll: P) -> Result<Self, PluginError> {
        let mut file = File::open(dll)?;
        let size = file.metadata()?.len() as usize;

        let mut blob = Blob::new_zeroed(size)?;

        file.read_exact(&mut blob)?;

        let yoke = Yoke::try_attach_to_cart(blob, |data| {
            let file = PeFile::from_bytes(data).context("failed to parse file")?;
            let by = file.exports()?.by()?;

            let dll_file = DllRef { file, exports: by };

            Ok::<_, PluginError>(dll_file)
        })?;

        Ok(Self(yoke))
    }

    pub fn is_symbol(&self, name: &str) -> bool {
        let file = self.0.get();
        file.exports.name(name).is_ok()
    }

    pub fn symbol_rva(&self, name: &str) -> Result<Rva, PluginError> {
        let file = self.0.get();
        let rva = file
            .exports
            .name(name)
            .ok()
            .and_then(|e| e.symbol())
            .ok_or(PluginError::SymbolNotFound)?;

        Ok(rva)
    }

    pub fn object(&self) -> DllRef<'_> {
        *self.0.get()
    }

    pub fn cart(&self) -> &[u8] {
        self.0.backing_cart()
    }
}

impl Deref for Dll {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Yokeable)]
pub struct DllRef<'a> {
    pub file: PeFile<'a>,
    pub exports: By<'a, PeFile<'a>>,
}
