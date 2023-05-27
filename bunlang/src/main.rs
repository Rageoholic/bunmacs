use std::{
    collections::hash_map::DefaultHasher,
    fmt::Debug,
    hash::{BuildHasher, BuildHasherDefault},
    io,
    num::NonZeroUsize,
};

use string_interner::{backend::Backend, symbol::SymbolUsize, DefaultBackend, StringInterner};

#[derive(Debug)]
#[non_exhaustive]
enum Expr<B: Backend> {
    Symbol(B::Symbol),
    Number(i64),
    List(Vec<Expr<B>>),
    Bool(bool),
}

impl<B: Backend> Clone for Expr<B> {
    fn clone(&self) -> Self {
        match self {
            Self::Symbol(arg0) => Self::Symbol(arg0.clone()),
            Self::Number(arg0) => Self::Number(arg0.clone()),
            Self::List(arg0) => Self::List(arg0.clone()),
            Self::Bool(b) => Self::Bool(*b),
        }
    }
}

#[derive(Debug)]
enum Err {
    UnmatchedCloser,
    UnmatchedOpeners { depth: NonZeroUsize },
}

// #[derive(Debug, Clone)]
// struct Env {
//     data: HashMap<SymbolUsize, Expr>,
// }

#[derive(Debug)]
struct InternTable<B: Backend, H: BuildHasher> {
    intern_table: StringInterner<B, H>,
    open_paren: B::Symbol,
    close_paren: B::Symbol,
    add_symbol: B::Symbol,
    sub_symbol: B::Symbol,
    mul_symbol: B::Symbol,
    div_symbol: B::Symbol,
    true_symbol: B::Symbol,
    false_symbol: B::Symbol,
    if_symbol: B::Symbol,
}

fn tokenize<B: Backend, H: BuildHasher>(
    expr: String,
    intern_table: &mut InternTable<B, H>,
) -> Vec<B::Symbol> {
    expr.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|x| intern_table.intern_table.get_or_intern(x))
        .collect()
}

fn parse<B: Backend, H: BuildHasher>(
    token_stream: Vec<B::Symbol>,
    intern_table: &mut InternTable<B, H>,
) -> Result<Vec<Expr<B>>, Vec<Err>> {
    let mut stack = vec![];
    let mut curr = Vec::with_capacity(token_stream.len());

    let mut errs = vec![];

    for symbol in token_stream {
        if symbol == intern_table.open_paren {
            stack.push(curr);
            curr = vec![];
        } else if symbol == intern_table.close_paren {
            if let Some(mut old) = stack.pop() {
                old.push(Expr::List(curr));
                curr = old;
            } else {
                errs.push(Err::UnmatchedCloser)
            }
        } else if symbol == intern_table.true_symbol {
            curr.push(Expr::Bool(true))
        } else if symbol == intern_table.false_symbol {
            curr.push(Expr::Bool(false))
        } else {
            let str = intern_table.intern_table.resolve(symbol).unwrap();
            match str.parse() {
                Ok(num) => curr.push(Expr::Number(num)),
                Result::Err(_) => curr.push(Expr::Symbol(symbol)),
            }
        }
    }

    match NonZeroUsize::new(stack.len()) {
        Some(depth) => errs.push(Err::UnmatchedOpeners { depth }),
        None => {}
    }

    if errs.len() != 0 {
        Err(errs)
    } else {
        Ok(curr)
    }
}

fn slurp_expr() -> String {
    let mut expr = String::new();

    io::stdin()
        .read_line(&mut expr)
        .expect("failed to read line");
    expr
}

fn main() {
    let mut interner: StringInterner<
        DefaultBackend<SymbolUsize>,
        BuildHasherDefault<DefaultHasher>,
    > = StringInterner::new();

    let mut intern_table = InternTable {
        open_paren: interner.get_or_intern("("),
        close_paren: interner.get_or_intern(")"),
        add_symbol: interner.get_or_intern("+"),
        sub_symbol: interner.get_or_intern("-"),
        mul_symbol: interner.get_or_intern("*"),
        div_symbol: interner.get_or_intern("/"),
        true_symbol: interner.get_or_intern("#t"),
        false_symbol: interner.get_or_intern("#f"),
        if_symbol: interner.get_or_intern("if"),
        intern_table: interner,
    };
    loop {
        println!("risp >");
        let expr = slurp_expr();
        let tokens = tokenize(expr, &mut intern_table);

        match parse(tokens, &mut intern_table) {
            Ok(exprs) => {
                for expr in exprs {
                    match eval(&expr, &mut intern_table) {
                        Ok(Expr::Number(n)) => println!("{}", n),
                        Ok(Expr::List(l)) => println!("{:?}", l),
                        Ok(Expr::Symbol(s)) => {
                            println!("#:{}", intern_table.intern_table.resolve(s).unwrap())
                        }
                        Ok(Expr::Bool(b)) => println!("{}", b),
                        Result::Err(err) => println!("ERROR: {err}"),
                    }
                }
            }
            Result::Err(errs) => {
                for err in errs {
                    match err {
                        Err::UnmatchedCloser => println!("Unmatched closing delimiter"),
                        Err::UnmatchedOpeners { depth } => {
                            println!("{} unmatched opening delimiter", depth)
                        }
                    }
                }
            }
        }
    }
}

fn eval<B: Backend, H: BuildHasher>(
    expr: &Expr<B>,
    intern_table: &mut InternTable<B, H>,
) -> Result<Expr<B>, String>
where
    B::Symbol: Clone,
{
    match expr {
        Expr::List(list) => call_fn(list, intern_table),
        _ => Ok(expr.clone()),
    }
}

fn call_fn<B: Backend, H: BuildHasher>(
    list: &Vec<Expr<B>>,
    intern_table: &mut InternTable<B, H>,
) -> Result<Expr<B>, String> {
    let mut iter = list
        .iter()
        .map(|e| eval(e, intern_table))
        .collect::<Vec<_>>()
        .into_iter();
    if let Expr::Symbol(sym) = iter.next().unwrap_or(Err("called empty list".to_owned()))? {
        if sym == intern_table.add_symbol {
            let mut sum = 0;
            for elem in iter {
                let elem = elem?;
                if let Expr::Number(num) = elem {
                    sum += num;
                } else if let Expr::List(_) = elem {
                    if let Expr::Number(num) = eval(&elem, intern_table)? {
                        sum += num;
                    } else {
                        return Err("Non number elem in math call".to_string());
                    }
                } else {
                    return Err("Non number elem in math function".to_string());
                }
            }
            Ok(Expr::Number(sum))
        } else if sym == intern_table.sub_symbol {
            match iter.len() {
                1 => {
                    let elem = iter
                        .next()
                        .expect("iter.len is incoherent with actual length?");
                    if let Expr::Number(n) = elem? {
                        Ok(Expr::Number(-n))
                    } else {
                        Err("Cannot negate a non-number".to_owned())
                    }
                }
                _ => {
                    let elem = iter
                        .next()
                        .expect("iter.len is incoherent with actual length?");
                    if let Expr::Number(mut res) = elem? {
                        for elem in iter {
                            if let Expr::Number(n) = elem? {
                                res -= n;
                            } else {
                                Err("Non number elem in math call")?
                            }
                        }
                        Ok(Expr::Number(res))
                    } else {
                        Err("Non number elem in math call".to_owned())
                    }
                }
            }
        } else if sym == intern_table.div_symbol {
            let elem = iter
                .next()
                .unwrap_or(Err("Called div on an empty list".to_owned()));
            if let Expr::Number(mut res) = elem? {
                for elem in iter {
                    if let Expr::Number(n) = elem? {
                        if n == 0 {
                            return Err("Divide by zero!".to_owned());
                        } else {
                            res /= n;
                        }
                    } else {
                        return Err("Non number in math function".to_owned());
                    }
                }
                Ok(Expr::Number(res))
            } else {
                return Err("Non number in math function".to_owned());
            }
        } else if sym == intern_table.mul_symbol {
            let mut res = 1;

            for elem in iter {
                if let Expr::Number(n) = elem? {
                    res *= n;
                } else {
                    return Err("Non number in math function".to_owned());
                }
            }
            Ok(Expr::Number(res))
        } else if sym == intern_table.if_symbol {
            if iter.len() == 3 {
                let expr_0 = iter.next().expect("incorrect iter.len");
                let expr_1 = iter.next().expect("incorrect iter.len")?;
                let expr_2 = iter.next().expect("incorrect iter.len")?;
                if let Expr::Bool(b) = expr_0? {
                    eval(if b { &expr_1 } else { &expr_2 }, intern_table)
                } else {
                    Err("Non boolean condition to if statement".to_owned())
                }
            } else {
                Err(format!("Expected 3 args found {} args", iter.len()).to_string())
            }
        } else {
            Err("Unknown op".into())
        }
    } else if list.len() > 0 {
        Err("Nonsymbol in head positon: Cannot call".to_owned())
    } else {
        Err("calling empty list".to_owned())
    }
}
