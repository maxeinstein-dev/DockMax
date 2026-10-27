//! The launcher's calculator (spec 0012 §5, L2): a query that is a sum shows
//! its result first. Its own parser, which only does sums: nothing typed is
//! ever run. Pure.
//!
//! Numbers take a comma or a point as the decimal mark (`1,5` and `1.5`);
//! `+ - * /`, `^` (power, from the right), `%` after a number (a hundredth:
//! `200*10%` is 20) and parentheses. A query is a sum only with at least one
//! of those operators between or after numbers: `-5` and `(5)` are not.

/// The deepest nesting of parentheses and signs looked at.
const MAX_DEPTH: usize = 32;

/// The result of `query` when it is a sum; `None` when it is not one, is
/// incomplete, or has no finite result (a division by zero).
pub fn evaluate(query: &str) -> Option<f64> {
    let tokens = tokenize(query)?;
    let operators = tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| match token {
            Token::Op(b'-' | b'+') => {
                *index > 0 && !matches!(tokens[index - 1], Token::Op(_) | Token::Open)
            }
            Token::Op(_) | Token::Percent => true,
            _ => false,
        })
        .count();
    if operators == 0 {
        return None;
    }
    let mut parser = Parser {
        tokens: &tokens,
        at: 0,
        depth: 0,
    };
    let value = parser.sum()?;
    (parser.at == tokens.len() && value.is_finite()).then_some(value)
}

/// `value` as the launcher shows it, with `decimal` as the decimal mark: at
/// most ten decimals, without the zeros at the end (`3,5`, `0,3333333333`).
pub fn format(value: f64, decimal: char) -> String {
    let value = if value == 0.0 { 0.0 } else { value };
    let size = value.abs();
    let text = if size >= 1e15 || (size > 0.0 && size < 1e-10) {
        format!("{value:e}")
    } else {
        let fixed = format!("{value:.10}");
        let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
        if trimmed == "-0" {
            "0".to_owned()
        } else {
            trimmed.to_owned()
        }
    };
    text.replace('.', &decimal.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Number(f64),
    /// `+ - * / ^`
    Op(u8),
    Percent,
    Open,
    Close,
}

fn tokenize(query: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = query.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' => {
                chars.next();
            }
            '0'..='9' | ',' | '.' => {
                let mut number = String::new();
                let mut mark = false;
                while let Some(&c) = chars.peek() {
                    match c {
                        '0'..='9' => number.push(c),
                        ',' | '.' if !mark => {
                            mark = true;
                            number.push('.');
                        }
                        _ => break,
                    }
                    chars.next();
                }
                if !number.bytes().any(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                tokens.push(Token::Number(number.parse().ok()?));
            }
            '+' | '-' | '*' | '/' | '^' => {
                tokens.push(Token::Op(c as u8));
                chars.next();
            }
            '×' => {
                tokens.push(Token::Op(b'*'));
                chars.next();
            }
            '÷' => {
                tokens.push(Token::Op(b'/'));
                chars.next();
            }
            '%' => {
                tokens.push(Token::Percent);
                chars.next();
            }
            '(' => {
                tokens.push(Token::Open);
                chars.next();
            }
            ')' => {
                tokens.push(Token::Close);
                chars.next();
            }
            _ => return None,
        }
    }
    Some(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }

    fn deeper(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth <= MAX_DEPTH).then_some(())
    }

    /// `product (('+' | '-') product)*`
    fn sum(&mut self) -> Option<f64> {
        let mut value = self.product()?;
        while let Some(Token::Op(op @ (b'+' | b'-'))) = self.peek() {
            self.at += 1;
            let right = self.product()?;
            value = if op == b'+' {
                value + right
            } else {
                value - right
            };
        }
        Some(value)
    }

    /// `signed (('*' | '/') signed)*`
    fn product(&mut self) -> Option<f64> {
        let mut value = self.signed()?;
        while let Some(Token::Op(op @ (b'*' | b'/'))) = self.peek() {
            self.at += 1;
            let right = self.signed()?;
            value = if op == b'*' {
                value * right
            } else {
                value / right
            };
        }
        Some(value)
    }

    /// `('-' | '+') signed | power`: `-2^2` is -4.
    fn signed(&mut self) -> Option<f64> {
        if let Some(Token::Op(op @ (b'+' | b'-'))) = self.peek() {
            self.at += 1;
            self.deeper()?;
            let value = self.signed()?;
            self.depth -= 1;
            return Some(if op == b'-' { -value } else { value });
        }
        self.power()
    }

    /// `percent ('^' signed)?`, from the right.
    fn power(&mut self) -> Option<f64> {
        let base = self.percent()?;
        if let Some(Token::Op(b'^')) = self.peek() {
            self.at += 1;
            self.deeper()?;
            let exponent = self.signed()?;
            self.depth -= 1;
            return Some(base.powf(exponent));
        }
        Some(base)
    }

    /// `primary '%'*`
    fn percent(&mut self) -> Option<f64> {
        let mut value = self.primary()?;
        while let Some(Token::Percent) = self.peek() {
            self.at += 1;
            value /= 100.0;
        }
        Some(value)
    }

    /// A number or a sum in parentheses.
    fn primary(&mut self) -> Option<f64> {
        match self.peek()? {
            Token::Number(value) => {
                self.at += 1;
                Some(value)
            }
            Token::Open => {
                self.at += 1;
                self.deeper()?;
                let value = self.sum()?;
                self.depth -= 1;
                if self.peek()? != Token::Close {
                    return None;
                }
                self.at += 1;
                Some(value)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(query: &str) -> Option<String> {
        evaluate(query).map(|value| format(value, ','))
    }

    #[test]
    fn operators_keep_their_precedence() {
        assert_eq!(evaluate("2+2*3"), Some(8.0));
        assert_eq!(evaluate("(2+2)*3"), Some(12.0));
        assert_eq!(evaluate("10-4-3"), Some(3.0));
        assert_eq!(evaluate("8/4/2"), Some(1.0));
        assert_eq!(evaluate("2^3^2"), Some(512.0), "the power from the right");
        assert_eq!(evaluate("-2^2"), Some(-4.0));
        assert_eq!(evaluate("2*-3"), Some(-6.0));
        assert_eq!(evaluate("3 × 4 ÷ 2"), Some(6.0));
    }

    #[test]
    fn a_comma_or_a_point_is_the_decimal_mark() {
        assert_eq!(evaluate("1,5+2"), Some(3.5));
        assert_eq!(evaluate("1.5+2"), Some(3.5));
        assert_eq!(evaluate(",5*2"), Some(1.0));
        assert_eq!(evaluate("1,5,2+1"), None, "two marks in a number");
        assert_eq!(evaluate(",+1"), None);
    }

    #[test]
    fn percent_is_a_hundredth() {
        assert_eq!(evaluate("200*10%"), Some(20.0));
        assert_eq!(evaluate("50%"), Some(0.5));
    }

    #[test]
    fn division_by_zero_and_incomplete_sums_have_no_result() {
        for query in ["1/0", "0/0", "2+", "(2+3", "2+3)", "*2", "2**3", "()", ""] {
            assert_eq!(evaluate(query), None, "{query:?}");
        }
    }

    #[test]
    fn a_query_without_an_operator_is_not_a_sum() {
        for query in ["2024", "-5", "+5", "(5)", "-(5)", "chrome", "g 2+2", "2+2a"] {
            assert_eq!(evaluate(query), None, "{query:?}");
        }
    }

    #[test]
    fn deep_nesting_is_refused() {
        let deep = format!("{}1+1{}", "(".repeat(40), ")".repeat(40));
        assert_eq!(evaluate(&deep), None);
        let signs = format!("1+{}1", "-".repeat(40));
        assert_eq!(evaluate(&signs), None);
        assert_eq!(evaluate("((1+1))*2"), Some(4.0));
    }

    #[test]
    fn results_show_with_the_regional_mark_and_without_trailing_zeros() {
        assert_eq!(shown("2+2*3").as_deref(), Some("8"));
        assert_eq!(shown("7/2").as_deref(), Some("3,5"));
        assert_eq!(shown("1/3").as_deref(), Some("0,3333333333"));
        assert_eq!(shown("0.1+0.2").as_deref(), Some("0,3"));
        assert_eq!(shown("-1*0").as_deref(), Some("0"));
        assert_eq!(format(3.5, '.'), "3.5");
        assert_eq!(format(1e20, ','), "1e20");
    }
}
