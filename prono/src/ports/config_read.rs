pub trait ReadConfig<T> {
    fn default_config_path(&self) -> std::path::PathBuf;
    fn read<P: AsRef<std::path::Path>>(&self, config: P) -> T;
}
