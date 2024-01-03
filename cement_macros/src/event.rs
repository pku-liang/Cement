use proc_macro2::TokenStream;
use syn::parse::Parse;
use syn::visit_mut::VisitMut;
use syn::*;

use crate::visitor::{HwVisitor, PunctratedExprWrapper};

#[derive(Debug)]
pub(crate) struct EventStmts {
  pub(crate) event_name: Option<Expr>,
  pub(crate) c: Option<Ident>,
  pub(crate) stmts: Vec<Stmt>,
  pub(crate) expr: Option<Expr>,
}

impl Parse for EventStmts {
  fn parse(input: parse::ParseStream) -> Result<Self> {
    let mut stmts = Vec::new();

    let (event_name, c) = if input.peek(token::Paren) {
      let content: parse::ParseBuffer<'_>;
      let _ = parenthesized!(content in input);
      let event_name: Option<Expr> =
        Some(content.parse().expect("Expr as the name/c of the event"));
      if input.peek(token::Lt) {
        let _: Token![<] = input.parse()?;
        let c: Ident = input.parse()?;
        let _: Token![>] = input.parse()?;
        let _: Token![=>] = input.parse()?;
        (event_name, Some(c))
      } else {
        let _: Token![=>] = input.parse()?;
        (event_name, None) // here, the parsed expr is acturally the c
      }
    } else if input.peek(token::Lt) {
      let _: Token![<] = input.parse()?;
      let c: Ident = input.parse()?;
      let _: Token![>] = input.parse()?;
      let _: Token![=>] = input.parse()?;
      (None, Some(c))
    } else {
      (None, None)
    };

    while !input.is_empty() {

      let forked = input.fork();
      let stmt = match input.parse::<Stmt>() {
        Ok(stmt) => stmt,
        Err(_) => {
          let expr: Expr = forked.parse()?;
          Stmt::Expr(expr, None)
        },
      };
      stmts.push(stmt);
    }

    if stmts.len() > 0 {
      let last_stmt = stmts.pop().unwrap();
      match last_stmt {
        Stmt::Expr(expr, None) => {
          Ok(EventStmts { event_name, c, stmts, expr: Some(expr) })
        },
        k @ _ => {
          stmts.push(k);
          Ok(EventStmts { event_name, c, stmts, expr: None })
        },
      }
    } else {
      Ok(EventStmts { event_name, c, stmts, expr: None })
    }
  }
}

pub(crate) fn event_transform(event_stmt: EventStmts) -> TokenStream {
  let EventStmts { event_name, c, stmts, expr } = event_stmt;
  let (event_name, c) = match (event_name, c) {
    (Some(event_name), Some(c)) => (event_name, c),
    _ => {
      // abort_call_site!("no event_name, c> provided")
      return TokenStream::default();
    },
  };


  let mut hw_visitor = HwVisitor::new(c.to_owned());
  let stmts: Vec<_> = stmts
    .into_iter()
    .map(|x| {
      let mut x = x;
      hw_visitor.visit_stmt_mut(&mut x);
      x
    })
    .collect();

  let add_event = if !stmts.is_empty() {
    quote! {
        let __event = #c.add_event(Some(#event_name.to_string()));
        let __region = #c.ir.add_region(Region::new(false));

        #c.begin_region(__region);
        #(
            #stmts
        )*
        #c.end_region();

        #c.add_event_when(&__event, __region);
        
    }
  } else if expr.is_none() {
    quote! {
        let __event = #c.add_event(Some(#event_name.to_string()));
    }
  } else {
    quote! {
        let __event = #c.add_event(Some(#event_name.to_string()));
    }
  };

  let ret_val: TokenStream = match expr {
    Some(expr) => {
      quote! {
          #c.specify_event_eq_signal(&__event, #expr);
          __event
      }
    },
    None => {
      quote! {
          __event
      }
    },
  };
  quote! {
      {
          #add_event
          #ret_val
      }
  }
}

pub(crate) fn event(input: TokenStream) -> TokenStream {
  let event_stmts: EventStmts = match parse2(input) {
    Ok(event_stmts) => event_stmts,
    Err(err) => {
      return err.to_compile_error();
    },
  };

  event_transform(event_stmts)
}

#[cfg(test)]
mod test {
  use std::str::FromStr;

  use proc_macro2::TokenStream;

  #[test]
  fn test_parse_transform() -> syn::Result<()> {
    let tokens = TokenStream::from_str(
      "
            (\"store\") =>
            [reg.wr_event] reg.wr %= module.i;
            is_odd.to_owned()
            ",
    )
    .unwrap();

    // println!("{}", tokens.to_string());
    let transformed = super::event(tokens);
    println!("{}", transformed);
    Ok(())
  }
}

fn guard_transform(args: Vec<Expr>) -> TokenStream {
  let mut args_iter = args.into_iter();
  let wire =
    args_iter.next().expect("the first arguement is expected to be the guarded wire");
  let name =
    args_iter.next().expect("the second arguement is expected to be the guard name");
  let c = args_iter.next().expect("The third argument is expected to be the cmtc");

  quote! {
      {
          let __guard = #c.add_event(Some(#name.to_string()));
          #c.add_event_guard(&__guard, #wire);
          __guard
      }
  }
}

pub(crate) fn guard(input: TokenStream) -> TokenStream {
  let punctuated_expr: PunctratedExprWrapper = match parse2(input) {
    Ok(punctuated_expr) => punctuated_expr,
    Err(err) => {
      return err.to_compile_error();
    },
  };

  guard_transform(punctuated_expr.punctuated.into_iter().collect())
}
