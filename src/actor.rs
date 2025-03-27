use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

// 自定义宏依赖
use proc_macro::TokenStream;
use quote::quote;
use syn;

// Context 结构体，类似 actix 的 Context
struct Context<A: Actor> {
    tx: SyncSender<Box<dyn MessageHandler<A>>>,
}

impl<A: Actor> Context<A> {
    fn new(tx: SyncSender<Box<dyn MessageHandler<A>>>) -> Self {
        Context { tx }
    }
}

// Actor trait，类似 actix 的 Actor
trait Actor: Sized + Send + 'static {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {}
    fn stopped(&mut self, _ctx: &mut Self::Context) {}
}

// Handler trait，类似 actix 的 Handler
trait Handler<M: Message>: Actor {
    type Result;

    fn handle(&mut self, msg: M, ctx: &mut Self::Context) -> Self::Result;
}

// Message trait
trait Message: Send + 'static {
    fn handle<A: Actor>(&self, actor: &mut A, ctx: &mut Context<A>);
}

// 动态分发的消息处理
trait MessageHandler<A: Actor>: Send {
    fn handle(&self, actor: &mut A, ctx: &mut Context<A>);
}

// 自定义 Message derive 宏
#[proc_macro_derive(Message)]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl crate::Message for #name {
            fn handle<A: crate::Actor + crate::Handler<Self>>(&self, actor: &mut A, ctx: &mut crate::Context<A>) {
                crate::Handler::<Self>::handle(actor, self.clone(), ctx);
            }
        }
    };

    TokenStream::from(expanded)
}

// 自定义 rtype 属性宏
#[proc_macro_attribute]
pub fn rtype(attr: TokenStream, item: TokenStream) -> TokenStream {
    let result_type = syn::parse_macro_input!(attr as syn::AttributeArgs);
    let item = syn::parse_macro_input!(item as syn::ItemStruct);
    let name = &item.ident;

    let result = result_type
        .into_iter()
        .find(|arg| match arg {
            syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => nv.path.is_ident("result"),
            _ => false,
        })
        .and_then(|arg| match arg {
            syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => match &nv.lit {
                syn::Lit::Str(lit_str) => Some(lit_str.value()),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| "()".to_string());

    let result_type: syn::Type = syn::parse_str(&result).unwrap();

    let expanded = quote! {
        #[derive(Clone)]
        #item

        impl<T: crate::Actor + crate::Handler<#name, Result = #result_type>> crate::MessageHandler<T> for #name {
            fn handle(&self, actor: &mut T, ctx: &mut crate::Context<T>) {
                crate::Handler::<Self>::handle(actor, self.clone(), ctx);
            }
        }
    };

    TokenStream::from(expanded)
}

// Addr 结构体，类似 actix 的 Addr
#[derive(Clone)]
struct Addr<A: Actor> {
    tx: SyncSender<Box<dyn MessageHandler<A>>>,
}

impl<A: Actor> Addr<A> {
    fn do_send<M>(&self, msg: M)
    where
        M: Message + Clone,
        A: Handler<M>,
    {
        self.tx.send(Box::new(msg)).unwrap();
    }
}
