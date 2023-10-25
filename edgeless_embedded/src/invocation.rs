// Temporary duplicate until https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html is done.
pub trait InvocationAPI {
    async fn handle(&mut self, event: edgeless_api_core::invocation::Event<&[u8]>)
        -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()>;
}

// https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/builder-provider-api.html#dynamic-dispatch-behind-the-api
pub trait InvocationAPIAPIDyn {
    fn handle<'a>(
        &'a mut self,
        event: edgeless_api_core::invocation::Event<&'a [u8]>,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<edgeless_api_core::invocation::LinkProcessingResult, ()>> + 'a>>;
}

impl<T: InvocationAPI> InvocationAPIAPIDyn for T {
    fn handle<'a>(
        &'a mut self,
        event: edgeless_api_core::invocation::Event<&'a [u8]>,
    ) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<edgeless_api_core::invocation::LinkProcessingResult, ()>> + 'a>>
    {
        alloc::boxed::Box::pin(<Self as InvocationAPI>::handle(self, event))
    }
}
