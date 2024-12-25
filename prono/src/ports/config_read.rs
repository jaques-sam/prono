use std::path::Path;

pub trait ReadConfig<T> {
    fn read(&self, config: &Path) -> T;
}
