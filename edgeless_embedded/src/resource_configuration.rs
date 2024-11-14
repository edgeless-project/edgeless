// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// Temporary duplicate until https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html is done.

#[allow(async_fn_in_trait)]
pub trait ResourceConfigurationAPI {
    async fn start(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse>;
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse>;
    async fn patch(
        &mut self,
        resource_id: edgeless_api_core::resource_configuration::EncodedPatchRequest,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse>;
}

// https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/builder-provider-api.html#dynamic-dispatch-behind-the-api
pub trait ResourceConfigurationAPIDyn {
    fn start<'a>(
        &'a mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> core::pin::Pin<
        alloc::boxed::Box<
            dyn core::future::Future<Output = Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse>> + 'a,
        >,
    >;
    fn stop(
        &mut self,
        resource_id: edgeless_api_core::instance_id::InstanceId,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<(), edgeless_api_core::common::ErrorResponse>> + '_>>;
    fn patch<'a>(
        &'a mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<(), edgeless_api_core::common::ErrorResponse>> + 'a>>;
}

impl<T: ResourceConfigurationAPI> ResourceConfigurationAPIDyn for T {
    fn start<'a>(
        &'a mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> core::pin::Pin<
        alloc::boxed::Box<
            dyn core::future::Future<Output = Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse>> + 'a,
        >,
    > {
        alloc::boxed::Box::pin(<Self as ResourceConfigurationAPI>::start(self, instance_specification))
    }
    fn stop(
        &mut self,
        resource_id: edgeless_api_core::instance_id::InstanceId,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<(), edgeless_api_core::common::ErrorResponse>> + '_>> {
        alloc::boxed::Box::pin(<Self as ResourceConfigurationAPI>::stop(self, resource_id))
    }
    fn patch<'a>(
        &'a mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<(), edgeless_api_core::common::ErrorResponse>> + 'a>> {
        alloc::boxed::Box::pin(<Self as ResourceConfigurationAPI>::patch(self, patch_req))
    }
}
