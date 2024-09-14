use proc_macro::TokenStream;
use winnow::ascii;
use winnow::combinator::repeat;
use winnow::error::InputError;
use winnow::prelude::*;
use winnow::token::one_of;
use winnow::Str;

#[proc_macro]
pub fn vif(input: TokenStream) -> TokenStream {
	let raw_input = input.to_string();

	let (_, upper_bits, _, lower_bits, ns) = (
		'E'.void(),
		ascii::digit1::<_, InputError<Str>>
			.map(|s| u8::from_str_radix(s, 2).expect("upper must be a valid binary expression")),
		' '.void(),
		ascii::digit0.map(|s: &str| {
			if !s.is_empty() {
				u8::from_str_radix(s, 2).expect("lower must be a valid binary expression")
			} else {
				0
			}
		}),
		repeat::<_, _, String, _, _>(0..=4, one_of(('n', 'p'))),
	)
		.parse(raw_input.as_str())
		.unwrap();

	let base = (upper_bits << 4) | (lower_bits << ns.len());

	let mask_inv = 0xFF << ns.len();
	let mask = !mask_inv;

	let range_start = base & mask_inv;
	let range_end = base | mask;

	if range_start == range_end {
		format!(r"{range_start}")
	} else {
		format!(r"{range_start}..={range_end}")
	}
	.parse()
	.unwrap()
}
