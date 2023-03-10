pub mod ast;

pub use ast::AST;
use lexer::Token;
use location::{Located};
use token_combinator::{
    alt, delimited, many0, many0_count, map, map_res, preceded, tuple,
    TokenParseError, TokenParseErrorKind, TokenParseResult, TokenParser,
};

type Tokens<'a> = &'a [Located<Token<'a>>];

type ParseResult<'a> = TokenParseResult<'a, Located<Token<'a>>, Located<AST<'a>>>;
type NotLocatedParseResult<'a> = TokenParseResult<'a, Located<Token<'a>>, AST<'a>>;

use lexer::token::parser::*;

fn located<'a>(
    mut parser: impl TokenParser<'a, Located<Token<'a>>, AST<'a>>,
) -> impl FnMut(&'a [Located<Token<'a>>]) -> TokenParseResult<'a, Located<Token<'a>>, Located<AST<'a>>>
{
    move |tokens: &'a [Located<Token<'a>>]| {
        let from = tokens[0].range;
        let (rest, output) = parser.parse(tokens)?;
        let to = rest.get(0).unwrap_or(tokens.last().unwrap()).range;
        Ok((
            rest,
            Located {
                range: (from.0, to.1),
                value: output,
            },
        ))
    }
}

fn parse_metadata(tokens: Tokens) -> ParseResult {
    located(map(preceded(hat, parse_form), |form| {
        AST::Metadata(Box::new(form))
    }))(tokens)
}

fn parse_symbol(tokens: Tokens) -> ParseResult {
    located(map(symbol, |symbol_str| {
        let splited = symbol_str.split('/').collect::<Vec<_>>();
        if splited.len() == 1 {
            let name = splited[0];
            return AST::Symbol(ast::Symbol { ns: None, name });
        } else if splited.len() == 2 {
            let ns = splited[0];
            let name = splited[1];
            return AST::Symbol(ast::Symbol { name, ns: Some(ns) });
        } else {
            unreachable!()
        }
    }))(tokens)
}

fn parse_atom_deref(tokens: Tokens) -> ParseResult {
    located(map(preceded(at, parse_symbol), |sym_ast| {
        if let AST::Symbol(sym) = sym_ast.value {
            AST::AtomDeref(sym)
        } else {
            unreachable!()
        }
    }))(tokens)
}

fn parse_unquoted_symbol(tokens: Tokens) -> ParseResult {
    located(map(preceded(tilde, parse_symbol), |sym_ast| {
        if let AST::Symbol(sym) = sym_ast.value {
            AST::Unquoted(sym)
        } else {
            unreachable!();
        }
    }))(tokens)
}

fn parse_unquoted_splicing_symbol(tokens: Tokens) -> ParseResult {
    located(map(preceded(tilde_at, parse_symbol), |sym_ast| {
        if let AST::Symbol(sym) = sym_ast.value {
            AST::UnquotedSplicing(sym)
        } else {
            unreachable!();
        }
    }))(tokens)
}

fn parse_and(tokens: Tokens) -> ParseResult {
    located(map(and, |_| AST::And))(tokens)
}

fn parse_keyword(tokens: Tokens) -> ParseResult {
    located(map(keyword, |keyword_str| {
        let name = if keyword_str.starts_with("::") {
            &keyword_str[2..]
        } else {
            &keyword_str[1..]
        };
        let splited = name.split('/').collect::<Vec<_>>();
        if splited.len() == 1 {
            let name = splited[0];
            return AST::Keyword(ast::Keyword { ns: None, name });
        } else if splited.len() == 2 {
            let ns = splited[0];
            let name = splited[1];
            return AST::Keyword(ast::Keyword { name, ns: Some(ns) });
        } else {
            unreachable!()
        }
    }))(tokens)
}

fn parse_char_literal(tokens: Tokens) -> ParseResult {
    located(map(char_literal, |c| AST::CharLiteral(*c)))(tokens)
}

fn parse_string_literal(tokens: Tokens) -> ParseResult {
    located(map(string_literal, |str| AST::StringLiteral(&str)))(tokens)
}

fn parse_integer_literal(tokens: Tokens) -> ParseResult {
    located(map(integer_literal, |i| AST::IntegerLiteral(*i)))(tokens)
}

fn parse_float_literal(tokens: Tokens) -> ParseResult {
    located(map(float_literal, |f| AST::FloatLiteral(*f)))(tokens)
}

fn parse_list(tokens: Tokens) -> ParseResult {
    located(map(
        delimited(l_paren, many0(parse_form), r_paren),
        |forms| AST::List(forms),
    ))(tokens)
}

fn parse_vector(tokens: Tokens) -> ParseResult {
    located(map(
        delimited(l_bracket, many0(parse_form), r_bracket),
        |forms| AST::Vector(forms),
    ))(tokens)
}

fn parse_map(tokens: Tokens) -> ParseResult {
    located(map_res(
        delimited(l_brace, many0(parse_form), r_brace),
        |res| match res {
            Ok((rest, kvs)) => {
                if kvs.len() % 2 != 0 {
                    return Err(TokenParseError {
                        errors: vec![TokenParseErrorKind::Other(
                            "map must have even number of forms".to_owned(),
                        )],
                        tokens_consumed: kvs.len(),
                    });
                }
                Ok((rest, AST::Map(kvs)))
            }
            Err(err) => Err(err),
        },
    ))(tokens)
}

fn parse_set(tokens: Tokens) -> ParseResult {
    located(map(
        tuple((sharp, delimited(l_brace, many0(parse_form), r_brace))),
        |(_, forms)| AST::Set(forms),
    ))(tokens)
}

fn parse_regex_literal(tokens: Tokens) -> ParseResult {
    located(map(preceded(sharp, string_literal), |str| {
        AST::RegexLiteral(&str)
    }))(tokens)
}

fn parse_anonymous_fn(tokens: Tokens) -> ParseResult {
    located(map(preceded(sharp, many0(parse_list)), |list| {
        AST::AnonymousFn(list)
    }))(tokens)
}

fn parse_quoted_form(tokens: Tokens) -> ParseResult {
    located(map(preceded(quote, parse_form), |form| {
        AST::Quoted(Box::new(form))
    }))(tokens)
}

fn parse_syntax_quoted_form(tokens: Tokens) -> ParseResult {
    located(map(preceded(syntax_quote, parse_form), |form| {
        AST::SyntaxQuoted(Box::new(form))
    }))(tokens)
}

pub fn parse_form(tokens: Tokens) -> ParseResult {
    alt((
        parse_symbol,
        parse_keyword,
        parse_char_literal,
        parse_string_literal,
        parse_integer_literal,
        parse_float_literal,
        parse_list,
        parse_vector,
        parse_map,
        parse_set,
        parse_regex_literal,
        parse_anonymous_fn,
        parse_metadata,
        parse_and,
        parse_atom_deref,
        parse_quoted_form,
        parse_unquoted_symbol,
        parse_unquoted_splicing_symbol,
        parse_syntax_quoted_form,
    ))(tokens)
}

pub fn parse_root(tokens: Tokens) -> NotLocatedParseResult {
    let mut rest = tokens;
    let mut forms = Vec::new();
    while !rest.is_empty() {
        let (rest_tokens, comment_out_count) = many0_count(sharp_underescore)(rest)?;
        rest = rest_tokens;
        for _ in 0..comment_out_count {
            if !rest.is_empty() {
                let (rest_tokens, _) = parse_form(rest)?;
                rest = rest_tokens;
            }
        }
        let (rest_tokens, form) = parse_form(rest)?;
        rest = rest_tokens;
        forms.push(form);
    }
    Ok((rest, AST::Root(forms)))
}
