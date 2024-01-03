pub trait ResourceProviderAPI {
    fn resource_configuration_api(&mut self) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI>;
}
