// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2
use winnow::error::{
	AddContext, ContextError, ErrMode, ErrorConvert, FromExternalError, ParserError, StrContext,
	StrContextValue,
};
use winnow::stream::Stream;

/// This is a now completely unnessary wrapper than I need to work out a smart way of replacing
#[derive(Debug, Clone, PartialEq)]
pub struct MBusError(ContextError<MBusContext>);

pub type MBResult<O> = Result<O, MBusError>;

impl MBusError {
	pub fn new() -> Self {
		Self(ContextError::new())
	}

	pub fn context(&self) -> impl Iterator<Item = &MBusContext> {
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
	type Inner = ContextError<MBusContext>;

	fn from_input(input: &I) -> Self {
		Self(Self::Inner::from_input(input))
	}

	fn into_inner(self) -> winnow::Result<Self::Inner, Self> {
		Ok(self.0)
	}

	fn assert(input: &I, message: &'static str) -> Self {
		let mut e = Self::Inner::assert(input, message);
		e.push(MBusContext::Assertion(message));
		Self(e)
	}
}

impl std::fmt::Display for MBusError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let ctx = &self.0;
		// If there's a cause, print it first
		if let Some(e) = ctx.cause() {
			std::fmt::Display::fmt(&e, f)?;
			writeln!(f)?;
		}
		let mut first = true;
		let mut level = 0;
		for context in ctx.context() {
			match context {
				MBusContext::Label(_)
				| MBusContext::ComputedLabel(_)
				| MBusContext::Assertion(_) => {
					if first {
						first = false;
					} else {
						level += 1;
					}
				}
				MBusContext::Expected(_) => (),
			}
			let padding = " ".repeat(level);
			writeln!(f, "{padding}{context}")?;
		}
		Ok(())
	}
}

impl<I: Stream> AddContext<I, StrContext> for MBusError {
	fn add_context(
		self,
		input: &I,
		token_start: &<I as Stream>::Checkpoint,
		context: StrContext,
	) -> Self {
		let new_context: MBusContext = context.into();
		self.add_context(input, token_start, new_context)
	}
}

impl<I: Stream> AddContext<I, MBusContext> for MBusError {
	fn add_context(
		self,
		input: &I,
		token_start: &<I as Stream>::Checkpoint,
		context: MBusContext,
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

impl ErrorConvert<MBusError> for ContextError<StrContext> {
	fn convert(self) -> MBusError {
		let mut new = ContextError::new();
		new.extend(self.context().cloned().map(|c| c.into()));
		MBusError(new)
	}
}

impl ErrorConvert<MBusError> for ContextError<MBusContext> {
	fn convert(self) -> MBusError {
		MBusError(self)
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MBusContext {
	/// Description of what is currently being parsed
	Label(&'static str),
	/// Computed description of what is currently being parsed
	ComputedLabel(String),
	/// Grammar item that was expected
	Expected(StrContextValue),
	/// Failed assertion
	Assertion(&'static str),
}

impl std::fmt::Display for MBusContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Label(name) => write!(f, "invalid {name}"),
			Self::ComputedLabel(name) => write!(f, "invalid {name}"),
			Self::Expected(value) => write!(f, "expected {value}"),
			Self::Assertion(text) => write!(f, "assertion failed: {text}"),
		}
	}
}

impl From<StrContext> for MBusContext {
	fn from(value: StrContext) -> Self {
		match value {
			StrContext::Label(l) => Self::Label(l),
			StrContext::Expected(e) => Self::Expected(e),
			unknown => unimplemented!("Unknown context variant {unknown}!"),
		}
	}
}
