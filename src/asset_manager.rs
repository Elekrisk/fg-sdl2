use std::{path::{Path, PathBuf}, collections::HashMap, any::Any};

pub struct AssetManager {
    loaded_assets: HashMap<PathBuf, Box<dyn Any>>
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            loaded_assets: HashMap::new()
        }
    }

    pub fn load<T: Asset>(&mut self, path: impl AsRef<Path>) -> Option<&T> {
        let pathbuf: PathBuf = path.as_ref().into();
        if self.loaded_assets.contains_key(&pathbuf) {
            self.loaded_assets[&pathbuf].downcast_ref()
        } else {
            let asset = T::load(path);
            self.loaded_assets.insert(pathbuf.clone(), Box::new(asset));
            Some(self.loaded_assets[&pathbuf].downcast_ref().unwrap())
        }
    }

    pub fn insert<T: 'static>(&mut self, path: impl AsRef<Path>, asset: T) {
        self.loaded_assets.insert(path.as_ref().into(), Box::new(asset));
    }

    pub fn get<T: 'static>(&self, path: impl AsRef<Path>) -> Option<&T> {
        self.loaded_assets.get::<PathBuf>(&path.as_ref().into()).and_then(|x| x.downcast_ref())
    }
}

pub trait Asset: 'static {
    fn load(path: impl AsRef<Path>) -> Self;
}

