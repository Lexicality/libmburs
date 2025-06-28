use winnow::error::ErrMode;
// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
#[allow(deprecated)]
use winnow::error::{
	AddContext, ContextError, ErrorConvert, FromExternalError, ParserError, StrContext,
};
use winnow::stream::Stream;

/// This is a now completely unnessary wrapper than I need to work out a smart way of replacing
#[allow(deprecated)]
#[derive(Debug, Clone, PartialEq)]
pub struct MBusError(ContextError<StrContext>);

pub type MBResult<O> = Result<O, MBusError>;

impl MBusError {
	pub fn new() -> Self {
		Self(ContextError::new())
	}

	pub fn context(&self) -> impl Iterator<Item = &StrContext> {
		self.0.context()
	}

	pub fn cause(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
		self.0.cause()
	}
}

impl Default for MBusError {
	fn default() -> Self {
		Self::new()
	}
}

impl<I: Stream> ParserError<I> for MBusError {
	type Inner = ContextError;

	fn from_input(input: &I) -> Self {
		Self(Self::Inner::from_input(input))
	}

	fn into_inner(self) -> winnow::Result<Self::Inner, Self> {
		Ok(self.0)
	}
}

impl std::fmt::Display for MBusError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl<I: Stream> AddContext<I, StrContext> for MBusError {
	fn add_context(
		self,
		input: &I,
		token_start: &<I as Stream>::Checkpoint,
		context: StrContext,
	) -> Self {
		Self(self.0.add_context(input, token_start, context))
	}
}

impl<I, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E> for MBusError {
	fn from_external_error(input: &I, e: E) -> Self {
		Self(ContextError::from_external_error(input, e))
	}
}

impl ErrorConvert<MBusError> for MBusError {
	fn convert(self) -> MBusError {
		self
	}
}

impl ErrorConvert<ErrMode<MBusError>> for MBusError {
	fn convert(self) -> ErrMode<MBusError> {
		ErrMode::Backtrack(self)
	}
}

// // impl<I: Stream> ErrorConvert<InputError<I>> for MBusError {
// impl<I: Stream + Clone> ErrorConvert<MBusError> for InputError<I> {
// 	fn convert(self) -> MBusError {
// 		#[allow(deprecated)]
// 		MBusError::from_error_kind(&self.input, self.kind)
// 	}
// }

impl ErrorConvert<MBusError> for ContextError<StrContext> {
	fn convert(self) -> MBusError {
		#[allow(deprecated)]
		MBusError(self)
	}
}
