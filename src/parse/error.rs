use std::error::Error;
use std::fmt::Display;

// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
use winnow::error::{
	AddContext, ContextError, ErrorConvert, ErrorKind, FromExternalError, InputError, ParserError,
	StrContext,
};
use winnow::stream::Stream;
use winnow::PResult;

/// Because the version of Winnow we're using doesn't let you use `ContextError`
/// with the bit-level parsers I've had to wrap it in a struct I control so I
/// can implement `ErrorConvert` and get it working again
#[derive(Debug, Clone, PartialEq)]
pub struct MBusError(ContextError<StrContext>, ErrorKind);

pub type MBResult<O> = PResult<O, MBusError>;

impl MBusError {
	pub fn new() -> Self {
		Self(ContextError::new(), ErrorKind::Fail)
	}

	pub fn context(&self) -> impl Iterator<Item = &StrContext> {
		self.0.context()
	}

	pub fn cause(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
		self.0.cause()
	}

	pub fn kind(&self) -> ErrorKind {
		self.1
	}
}

impl Default for MBusError {
	fn default() -> Self {
		Self::new()
	}
}

impl<I: Stream> ParserError<I> for MBusError {
	fn append(self, input: &I, token_start: &<I as Stream>::Checkpoint, kind: ErrorKind) -> Self {
		Self(self.0.append(input, token_start, kind), kind)
	}

	fn from_error_kind(input: &I, kind: ErrorKind) -> Self {
		Self(ContextError::from_error_kind(input, kind), kind)
	}
}

impl std::fmt::Display for MBusError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.1, self.0)
	}
}

impl<I: Stream> AddContext<I, StrContext> for MBusError {
	fn add_context(
		self,
		input: &I,
		token_start: &<I as Stream>::Checkpoint,
		context: StrContext,
	) -> Self {
		Self(self.0.add_context(input, token_start, context), self.1)
	}
}

impl<I, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E> for MBusError {
	fn from_external_error(input: &I, kind: ErrorKind, e: E) -> Self {
		Self(ContextError::from_external_error(input, kind, e), kind)
	}
}

impl ErrorConvert<MBusError> for MBusError {
	fn convert(self) -> MBusError {
		self
	}
}

// impl<I: Stream> ErrorConvert<InputError<I>> for MBusError {
impl<I: Stream + Clone> ErrorConvert<MBusError> for InputError<I> {
	fn convert(self) -> MBusError {
		MBusError::from_error_kind(&self.input, self.kind)
	}
}

impl ErrorConvert<MBusError> for ContextError<StrContext> {
	fn convert(self) -> MBusError {
		MBusError(self, ErrorKind::Fail)
	}
}

#[derive(Debug)]
pub enum MBusEncodErrorCause {
	NotImplementedYet,
	ReservedValue,
	UserDataTooLong,
}

#[derive(Debug)]
pub struct MBusEncodeError(pub MBusEncodErrorCause);

impl Display for MBusEncodeError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// TODO
		write!(f, "MBus Encode Error: {:?}", self.0)
	}
}

impl Error for MBusEncodeError {}
