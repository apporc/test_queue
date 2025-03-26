use ringbuf::{CachingCons, CachingProd, SharedRb, StaticRb, storage::Owning, traits::*};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct Queue<T, const N: usize> {
    _marker: std::marker::PhantomData<T>,
}

impl<T, const N: usize> Queue<T, N> {
    pub fn new() -> (Sender<T, N>, Receiver<T, N>) {
        let rb = StaticRb::<T, N>::default();
        let notify = Arc::new(Notify::new());

        let (prod, cons) = rb.split();

        (
            Sender {
                prod,
                notify: Arc::clone(&notify),
            },
            Receiver { cons, notify },
        )
    }
}

pub struct Sender<T, const N: usize> {
    prod: CachingProd<Arc<SharedRb<Owning<[std::mem::MaybeUninit<T>; N]>>>>,
    notify: Arc<Notify>,
}

impl<T, const N: usize> Sender<T, N> {
    #[inline(always)]
    pub fn send(&mut self, item: T) -> Result<(), T> {
        match self.prod.try_push(item) {
            Ok(()) => {
                self.notify.notify_one();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

pub struct Receiver<T, const N: usize> {
    cons: CachingCons<Arc<SharedRb<Owning<[std::mem::MaybeUninit<T>; N]>>>>,
    notify: Arc<Notify>,
}

impl<T, const N: usize> Receiver<T, N> {
    #[inline(always)]
    pub async fn recv(&mut self) -> T {
        loop {
            if let Some(item) = self.cons.try_pop() {
                return item;
            }
            self.notify.notified().await;
        }
    }
}
