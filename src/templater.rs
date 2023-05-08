use onlyerror::Error;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Error)]
pub enum Error {
	#[error("expected utf8")]
	Utf8(#[from] std::str::Utf8Error),

	#[error("templater formatter error")]
	Fmt(#[from] std::fmt::Error),

	#[error("variable {0} not found")]
	VarNotFound(String),

	#[error("missing `}}`")]
	MissingRightBrace,

	#[error("`{0}` is a reserved keyword")]
	ReservedKeyword(String),

	#[error("empty template expression")]
	ExpectedExpr,

	#[error("invalid template expression syntax")]
	InvalidExprSyntax,
}

pub fn template(template: &str, vars: HashMap<String, TemplateVar>) -> Result<String, Error> {
	validate_vars(&vars)?;

	let mut out = String::with_capacity(((template.len() as f64) * 1.5) as usize);

	let mut idx = 0;
	let template = template.as_bytes();
	while idx < template.len() {
		if template[idx] != b'{' {
			out.push(template[idx] as char);
			idx += 1;
			continue;
		}

		if idx < template.len() - 1 && template[idx + 1] == b'{' {
			out.push(template[idx] as char);
			idx += 2;
			continue;
		}

		let right_brace_offset = template[idx..]
			.iter()
			.position(|b| *b == b'}')
			.ok_or(Error::MissingRightBrace)?;

		let template_expr =
			std::str::from_utf8(&template[(idx + 1)..(idx + right_brace_offset)])?.trim();

		let expr = parse_expr(template_expr)?;

		match expr {
			Expr::VarAccess(name) => {
				let var = vars
					.get(name)
					.ok_or_else(|| Error::VarNotFound(name.to_string()))?;

				match var {
					TemplateVar::String(v) => {
						write!(&mut out, "{v}")?;
					}
					TemplateVar::Int(v) => {
						write!(&mut out, "{v}")?;
					}
					TemplateVar::Float(v) => {
						write!(&mut out, "{v}")?;
					}
					_ => unimplemented!("{var:?}"),
				}
			}
			Expr::ForEach(_) => unimplemented!(),
			Expr::EndFor => unimplemented!(),
		}

		idx += right_brace_offset + 1;
	}

	Ok(out)
}

fn validate_vars(vars: &HashMap<String, TemplateVar>) -> Result<(), Error> {
	let keywords = ["foreach", "endfor"];
	for var_name in vars.keys() {
		if keywords.contains(&var_name.as_str()) {
			return Err(Error::ReservedKeyword(var_name.to_string()));
		}
	}
	Ok(())
}

enum Expr<'a> {
	VarAccess(&'a str),
	ForEach(&'a str),
	EndFor,
}

fn parse_expr(expr: &str) -> Result<Expr<'_>, Error> {
	let word_end_idx = expr
		.as_bytes()
		.iter()
		.position(u8::is_ascii_whitespace)
		.unwrap_or(expr.len());

	let word = &expr[..word_end_idx];

	match word {
		"foreach" => {
			todo!()
		}
		"endfor" => Ok(Expr::EndFor),
		_ => {
			if word != expr {
				return Err(Error::InvalidExprSyntax);
			}
			Ok(Expr::VarAccess(word))
		}
	}
}

// TODO Idea for the future: instead of having ownership of everything, we could try a trait based
// 	approach and pass everything by reference to avoid copying.
#[derive(Debug)]
pub enum TemplateVar {
	Object(HashMap<String, TemplateVar>),
	Vec(Vec<TemplateVar>),
	String(String),
	Int(i64),
	Float(f64),
}

impl From<HashMap<String, TemplateVar>> for TemplateVar {
	fn from(v: HashMap<String, TemplateVar>) -> Self {
		TemplateVar::Object(v)
	}
}
impl From<Vec<TemplateVar>> for TemplateVar {
	fn from(v: Vec<TemplateVar>) -> Self {
		TemplateVar::Vec(v)
	}
}
impl From<String> for TemplateVar {
	fn from(v: String) -> Self {
		TemplateVar::String(v)
	}
}
impl From<i64> for TemplateVar {
	fn from(v: i64) -> Self {
		TemplateVar::Int(v)
	}
}
impl From<f64> for TemplateVar {
	fn from(v: f64) -> Self {
		TemplateVar::Float(v)
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	#[test]
	fn template_int() {
		let template = "Hello world!\n\
		We've already had { visitors } visits!";

		let expected = "Hello world!\n\
		We've already had 143 visits!";

		let mut vars = HashMap::new();
		vars.insert("visitors".to_string(), 143_i64.into());
		let result = super::template(template, vars).unwrap();
		assert_eq!(result, expected);
	}

	#[test]
	fn template_array_of_ints() {
		let template = "Hello world!\n\
		{ foreach nums }\n\
		Hello number { _ }!\n\
		Hello number { _ }!\n\
		Hello number { _ }!\n\
		Hello number { _ }!\n\
		{ endfor }";

		let expected = "Hello world!\n\
		{ foreach nums }\n\
		Hello number 143!\n\
		Hello number 13!\n\
		Hello number 3!\n\
		Hello number 19999!\n\
		{ endfor }";

		let mut vars = HashMap::new();
		vars.insert(
			"nums".to_string(),
			vec![143.into(), 13.into(), 3.into(), 19999.into()].into(),
		);
		let result = super::template(template, vars).unwrap();
		assert_eq!(result, expected);
	}
}
