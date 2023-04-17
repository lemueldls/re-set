use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_error::{abort, proc_macro_error, ResultExt};
use quote::quote;
use re_set::{
    parse::{CasePattern, Compiler},
    StateMachine,
};
use syn::{
    parse,
    parse::{Parse, ParseBuffer, ParseStream},
    LitInt, LitStr, Result, Token, Visibility,
};

struct Expressions {
    vis: Option<Visibility>,
    ident: Ident,
    exprs: Vec<LitStr>,
}

impl Parse for Expressions {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse::<Visibility>().ok();

        input.parse::<Token![fn]>()?;

        let ident = input.parse::<Ident>()?;

        let exprs = input
            .parse_terminated::<_, Token![,]>(|input| ParseBuffer::parse(input))?
            .into_iter()
            .collect();

        Ok(Self { vis, ident, exprs })
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn find(input: TokenStream) -> TokenStream {
    let Expressions { vis, ident, exprs } = parse(input).unwrap_or_abort();

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
        .unwrap_or_else(|error| abort!(ident, error));

    let patterns = StateMachine::new(&program);

    let max_size = patterns.step_size();
    let u_shrink = |n| LitInt::new(&format!("{n}u{max_size}"), Span::call_site());

    let u_first = u_shrink(patterns.first_step());

    let step_matches = patterns
        .steps
        .into_iter()
        .map(|(position, step_cases)| {
            let char_matches = step_cases.into_iter().map(|step_case| {
                let (start, end) = step_case.byte_range;

                match step_case.next_case {
                    CasePattern::Step(next_step, conditions) => {
                        let u_next = u_shrink(next_step);

                        if patterns.ends.contains_key(&next_step) {
                            quote! {
                                #start..=#end => {
                                    last_match = (#u_next, i);
                                    step = #u_next;
                                }
                            }
                        } else {
                            let conditions = conditions.into_iter().map(|(step, (start, end))| {
                                let u_step = u_shrink(step);

                                quote! {
                                    if (#start..=#end).contains(&next) {
                                        last_match = (#u_step, i);
                                    }
                                }
                            });

                            quote! {
                                #start..=#end => {
                                    #(#conditions)*

                                    step = #u_next
                                }
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

            let u_position = u_shrink(position);

            let default = patterns.ends.get(&position).map_or_else(
                || quote!(break),
                |match_index| quote!(return Some((#match_index, &input[..i]))),
            );

            quote! {
                #u_position => match next {
                    #(#char_matches,)*
                    _ => #default
                }
            }
        })
        .collect::<Vec<_>>();

    let end_matches = patterns.ends.iter().map(|(step, match_index)| {
        let u_step = u_shrink(*step);

        quote! {
            #u_step => Some((#match_index, &input[..=last_match.1]))
        }
    });

    let expanded = quote! {
        #[inline]
        #vis fn #ident(input: &str) -> Option<(usize, &str)> {
            let mut last_match = (#u_first, 0);
            let mut step = #u_first;

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
