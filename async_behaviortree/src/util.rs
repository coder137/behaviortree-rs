pub fn yield_now() -> impl Future<Output = ()> {
    Yield { done: false }
}

struct Yield {
    done: bool,
}

impl std::future::Future for Yield {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.done {
            return std::task::Poll::Ready(());
        }

        self.done = true;
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::util::yield_now;

    #[test]
    fn test_yield_now() {
        let executor = TickedAsyncExecutor::default();

        executor
            .spawn_local("", async {
                yield_now().await;
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);

        executor.tick(0.1, None);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(0.1, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
