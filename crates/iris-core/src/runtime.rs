//! Iris 异步运行时封装
//!
//! 基于 Tokio 的多线程运行时，提供跨平台的异步任务调度。
//! 主模块逻辑已内联在 [`crate::Context`] 中，此模块提供额外工具函数。

#![warn(missing_docs)]

use std::future::Future;

/// 在 Iris 运行时句柄上 spawn 一个异步任务。
///
/// 需要传入 [`tokio::runtime::Handle`]，通常通过 [`crate::Context::handle`] 获取。
pub fn spawn<F>(handle: &tokio::runtime::Handle, future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    handle.spawn(future)
}

/// 在 Iris 运行时句柄上阻塞执行一个异步任务。
pub fn block_on<F>(handle: &tokio::runtime::Handle, future: F) -> F::Output
where
    F: Future,
{
    handle.block_on(future)
}
