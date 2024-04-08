//! Utilities for working with error_stack::Report

use std::{
    any::Any,
    backtrace::{Backtrace, BacktraceStatus},
    fmt::Display,
};

use error_stack::{AttachmentKind, Context, Frame, FrameKind, Report};
use smallvec::SmallVec;
use tracing_error::SpanTraceStatus;

/// An [error_stack::Context] with its associated attachments
pub struct ContextWithAttachments<'a> {
    /// The context
    pub context: &'a dyn Context,
    /// The attachments associated with the context
    pub attachments: SmallVec<[AttachmentKind<'a>; 1]>,
}

/// Extension methods for [error_stack::Report]
pub trait ContextWithAttachmentsExt<'a, I: Iterator<Item = &'a Frame>> {
    /// Return an iterator of each context with its attachments
    fn by_error(self) -> ContextWithAttachmentsIterator<'a, I>;
}

impl<'a, I> ContextWithAttachmentsExt<'a, I> for I
where
    I: Iterator<Item = &'a Frame>,
{
    fn by_error(self) -> ContextWithAttachmentsIterator<'a, I> {
        ContextWithAttachmentsIterator { parent: self }
    }
}

/// An iterator that groups each [error_stack::Context] with its associated attachments
pub struct ContextWithAttachmentsIterator<'a, I: Iterator<Item = &'a Frame>> {
    parent: I,
}

impl<'a, I> Iterator for ContextWithAttachmentsIterator<'a, I>
where
    I: Iterator<Item = &'a Frame>,
{
    type Item = ContextWithAttachments<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut attachments: SmallVec<[AttachmentKind<'a>; 1]> = SmallVec::new();

        while let Some(frame) = self.parent.next() {
            match frame.kind() {
                FrameKind::Context(c) => {
                    attachments.reverse();

                    return Some(ContextWithAttachments {
                        context: c,
                        attachments,
                    });
                }
                FrameKind::Attachment(a) => {
                    attachments.push(a);
                }
            }
        }

        None
    }
}

/// Extract information from an [error_stack::Report]
pub struct ErrorStackInformation<'a> {
    /// The backtrace, if present
    pub backtrace: Option<&'a Backtrace>,
    /// The spantrace, if present
    pub spantrace: Option<&'a tracing_error::SpanTrace>,
}

impl<'a> ErrorStackInformation<'a> {
    /// Create an [ErrorStackInformation] from an [error_stack::Report]
    pub fn new(err: &'a Report<impl Context>) -> Self {
        let backtrace = err
            .downcast_ref::<Backtrace>()
            .filter(|b| b.status() == BacktraceStatus::Captured);
        let spantrace = err
            .downcast_ref::<tracing_error::SpanTrace>()
            .filter(|s| s.status() == SpanTraceStatus::CAPTURED);

        Self {
            backtrace,
            spantrace,
        }
    }
}
