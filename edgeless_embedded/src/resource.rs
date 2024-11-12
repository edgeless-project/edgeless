// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub mod epaper_display;
pub mod mock_display;
pub mod mock_sensor;
pub mod scd30_sensor;

#[allow(async_fn_in_trait)]
pub trait Resource: crate::invocation::InvocationAPI + crate::resource_configuration::ResourceConfigurationAPI {
    fn provider_id(&self) -> &'static str;
    fn resource_class(&self) -> &'static str;
    fn outputs(&self) -> &'static [&'static str];
    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool;
    async fn launch(&mut self, spawner: embassy_executor::Spawner, dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle);
}

// https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/builder-provider-api.html#dynamic-dispatch-behind-the-api
pub trait ResourceDyn: crate::resource_configuration::ResourceConfigurationAPIDyn + crate::invocation::InvocationAPIAPIDyn {
    fn provider_id(&self) -> &'static str;
    fn resource_class(&self) -> &'static str;
    fn outputs(&self) -> &'static [&'static str];

    fn has_instance<'a>(
        &'a self,
        id: &'a edgeless_api_core::instance_id::InstanceId,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = bool> + 'a>>;

    fn launch(
        &mut self,
        spawner: embassy_executor::Spawner,
        dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>>;
}

impl<T: Resource> ResourceDyn for T {
    fn provider_id(&self) -> &'static str {
        <Self as Resource>::provider_id(self)
    }

    fn resource_class(&self) -> &'static str {
        <Self as Resource>::resource_class(self)
    }

    fn outputs(&self) -> &'static [&'static str] {
        <Self as Resource>::outputs(self)
    }

    fn has_instance<'a>(
        &'a self,
        id: &'a edgeless_api_core::instance_id::InstanceId,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = bool> + 'a>> {
        alloc::boxed::Box::pin(<Self as Resource>::has_instance(self, id))
    }

    fn launch(
        &mut self,
        spawner: embassy_executor::Spawner,
        dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(<Self as Resource>::launch(self, spawner, dataplane_handle))
    }
}
