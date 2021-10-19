#![feature(proc_macro_diagnostic)]
use std::panic;
use proc_macro::TokenStream;
use syn::{Expr, ExprBinary, ExprBlock, ExprType, Ident, Lit, Path, Token, Type, braced, parse::{Parse, ParseStream}, parse_macro_input, punctuated::Punctuated, spanned::Spanned};
use quote::{format_ident, quote};

#[derive(Clone)]
struct Transition {
    next_state: Ident,
    transition_check: ExprBinary
}

impl Parse for Transition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let next_state: Ident = input.parse()?;
        input.parse::<Token![->]>()?;
        let transition_check_expr: Expr = input.parse()?;
        let transition_check: ExprBinary = match transition_check_expr {
            Expr::Binary(expr_bin) => {
                expr_bin
            },
            _ => {
                transition_check_expr.span().unwrap().error("Expected binary expr");
                panic!("Expected a boolean expression");
            }
        };

        Ok(Transition {
            next_state: next_state,
            transition_check: transition_check
        })
    }
}

#[derive(Clone)]
struct State {
    name: Ident,
    transitions: Vec<Transition>
}

impl Parse for State {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let transitions: Vec<Transition> = (Punctuated::<Transition, Token![,]>::parse_terminated(&content)?).into_iter().collect();

        Ok(State{
            name: name,
            transitions: transitions
        })
    }
}

struct AnimationGraph {
    name: Ident,
    params: Vec<ExprType>,
    states: Vec<State>
}

impl Parse for AnimationGraph {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        let _ = braced!(content in input);
        let params: Vec<ExprType> = (Punctuated::<ExprType, Token![,]>::parse_terminated(&content)?).into_iter().collect();
        input.parse::<Token![,]>()?;
        let states: Vec<State> = (Punctuated::<State, Token![,]>::parse_terminated(&input)?).into_iter().collect();

        Ok(AnimationGraph {
            name,
            params,
            states,
        })
    }
}

#[proc_macro]
pub fn animation_graph(input: TokenStream) -> TokenStream {
    let AnimationGraph { 
        name, 
        params, 
        states 
    } = parse_macro_input!(input as AnimationGraph);

    let state_idents: Vec<Ident> = states.clone().into_iter().map(|state| {
        state.name
    }).collect();

    let param_types: Vec<Type> = params.clone().into_iter().map(|param| {
        *param.ty
    }).collect();

    let param_names: Vec<Ident> = params.clone().into_iter().map(|param| {
        let temp = if let Expr::Path(path) = *param.expr {
            path.path.segments[0].ident.clone()
        } else {
            (*param.expr).span().unwrap().error("Expected ident here");
            panic!("Expected ident");
        };

        temp
    }).collect();

    let enum_ident = format_ident!("{}AnimationUpdate", name);
    let lower_name_ident = format_ident!("{}", name.to_string().to_lowercase());
    let system_ident = format_ident!("{}_animation_update", lower_name_ident);
    let query_ident = format_ident!("{}_query", lower_name_ident);
    let enum_query_for_ident = format_ident!("{}_action", lower_name_ident);

    // let state_paths: Vec<Ident> = states.clone().into_iter().map(|state| {
    //     format_ident!("{}::{}::{}", name, enum_ident, state.name)
    // }).collect();

    let states_match_statment: Vec<proc_macro2::TokenStream> = states.clone().into_iter().map(|state|{
        let state_name = state.name;
        let state_name_arm: proc_macro2::TokenStream = quote! {
            #enum_ident::#state_name
        }.into();

        let transition_ifs: proc_macro2::TokenStream = state.transitions.into_iter().map(|transition|{
            let next_state= transition.next_state;
            let next_state_path: proc_macro2::TokenStream = quote! {
                #enum_ident::#next_state
            }.into();
            let transition_check = transition.transition_check;

            quote! {
                if #transition_check {
                    *#enum_query_for_ident = #next_state_path;
                }
            }
        }).collect();

        quote! {
            #state_name_arm => {
                #transition_ifs
            },
        }
    }).collect();

    let expanded = quote! {
        mod #name {
            #[derive(Debug)]
            pub enum #enum_ident {
                #(#state_idents,)*
            }

            pub fn #system_ident (
                mut #query_ident: bevy::ecs::system::Query<(&mut #enum_ident, #(&#param_types,)*)>
            ) {
                for (mut #enum_query_for_ident, #(#param_names,)*) in #query_ident.iter_mut() {
                    println!("In here {:?}", #enum_query_for_ident);
                    match *#enum_query_for_ident {
                        #(#states_match_statment)*
                    }
                }
            }
        }
    };

    return expanded.into();
}