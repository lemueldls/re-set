use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro_error::{abort, proc_macro_error, ResultExt};
use quote::quote;
use re_set::{
    parse_program,
    state::{CasePattern, Compiler},
    ParsedProgram,
};
use syn::{
    parse,
    parse::{Parse, ParseStream},
    LitInt, LitStr, Result, Token,
};

struct Expressions {
    ident: Ident,
    exprs: Vec<LitStr>,
}

impl Parse for Expressions {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        input.parse::<Token![|]>()?;

        let mut exprs = Vec::new();

        loop {
            let expr = input.parse()?;
            exprs.push(expr);

            if input.is_empty() {
                break;
            }

            input.parse::<Token![|]>()?;
        }

        Ok(Self { ident, exprs })
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn find(input: TokenStream) -> TokenStream {
    let Expressions { ident, exprs } = parse(input).unwrap_or_abort();

    let compiler = Compiler::new().bytes(true);

    let program = compiler
        .compile(
            &exprs
                .iter()
                .map(|lit_str| {
                    regex_syntax::Parser::new()
                        .parse(&lit_str.value())
                        .unwrap_or_else(|error| abort!(lit_str, error))
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();

    let ParsedProgram { steps, ends } = parse_program(&program);

    let size = (2_u8 << (steps.len() / 256)) * 4;
    let u_shrink = |n| LitInt::new(&format!("{n}u{size}"), proc_macro2::Span::call_site());

    let step_matches = steps
        .into_iter()
        .map(|(position, step_cases)| {
            let char_matches = step_cases.into_iter().map(|step_case| {
                let start = step_case.char_range.start();
                let end = step_case.char_range.end();

                match step_case.next_case {
                    CasePattern::Step(next_step) => {
                        let u_next = u_shrink(next_step);

                        if ends.contains_key(&next_step) {
                            quote! {
                                #start..=#end => {
                                    last_match = (#u_next, i);
                                    step = #u_next;
                                }
                            }
                        } else {
                            quote! {
                                #start..=#end => step = #u_next
                            }
                        }
                    }
                    CasePattern::Match(match_index) => {
                        quote! {
                            #start..=#end => return Some((#match_index, &input[..=i]))
                        }
                    }
                }
            });

            let default = if let Some(match_index) = ends.get(&position) {
                quote!(Some((#match_index, &input[..i])))
            } else {
                quote!(None)
            };

            let u_position = u_shrink(position);

            quote! {
                #u_position => match next {
                    #(#char_matches,)*
                    _ => return #default
                }
            }
        })
        .collect::<Vec<_>>();

    let end_matches = ends.iter().map(|(step, match_index)| {
        let u_step = u_shrink(*step);

        quote! {
            #u_step => Some((#match_index, &input[..=last_match.1]))
        }
    });

    let expanded = quote! {
        #[inline]
        fn #ident(input: &str) -> Option<(usize, &str)> {
            let mut last_match = (0, 0);
            let mut step = 0;

            for (i, next) in input.as_bytes().iter().enumerate() {
                match step {
                    #(#step_matches,)*
                    _ => unreachable!()
                }
            }

            match last_match.0 {
                #(#end_matches,)*
                _ => None,
            }
        }
    };

    TokenStream::from(expanded)
}
