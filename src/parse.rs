use std::io::BufReader;
use serialize::json::Json;
use regex::Regex;

use self::Token::{TokSimpleExp, TokNoEscapeExp, TokCommentExp, TokBlockExp, TokBlockElseCond, TokBlockEndExp, TokPartialExp, TokRaw};
use self::HBToken::{TokPathEntry,TokNoWhiteSpaceBefore, TokNoWhiteSpaceAfter,TokStringParam,TokParamStart, TokParamSep, TokOption, TokLeadingWhiteSpace, TokTrailingWhiteSpace};

#[derive(Debug)]
pub enum Token {
  // base template tokens
  TokSimpleExp(String),
  TokNoEscapeExp(String),
  TokCommentExp(String),
  TokPartialExp(String,),
  TokBlockExp(String, bool),
  TokBlockElseCond(String),
  TokBlockEndExp(String,),
  TokRaw(String),
}

#[derive(Debug)]
pub enum HBToken {
  TokPathEntry(String),
  TokNoWhiteSpaceBefore,
  TokNoWhiteSpaceAfter,
  TokStringParam(String),
  TokParamStart,
  TokParamSep,
  TokOption(String),
  TokLeadingWhiteSpace(String),
  TokTrailingWhiteSpace(String),
}

rustlex! HandleBarsLexer {
  // expression definitions

  let OPEN  = "{{" '~'?;
  let CLOSE = [' ''\t']* '~'? "}}";
  let EXP = ('}'?[^'}'])*;

  let NEW_LINE     = (['\n'] | ['\r']['\n']);
  let IGN_WP        = [' ''\t''\r']*;
  let ALL_WP        = (NEW_LINE | IGN_WP)*;
  let PASS_THROUGH  = ALL_WP* ('{'?[^'{'' ''\t''\r''\n''\\'])*;
  let PASS_ESC      = '\\';
  let ESCAPED_EXP   = '\\' '{';
  let ESCAPED_ESC   = '\\' '\\';
  let ESCAPED_SKIP  = '\\' '\\'? [^'{''\\''\r''\n'];

  let BLOCK_EXP     = ALL_WP OPEN '#' EXP CLOSE ALL_WP;
  let BLOCK_INV_EXP = ALL_WP OPEN '^' EXP CLOSE ALL_WP;
  let END_EXP       = ALL_WP OPEN '/' EXP CLOSE   ALL_WP;
  let NO_ESC_EXP    = ALL_WP OPEN ('{' EXP '}' | '&' EXP) CLOSE ALL_WP;
  let PARTIAL_EXP   = ALL_WP OPEN '>' EXP CLOSE ALL_WP;
  let SIMPLE_EXP    = ALL_WP OPEN [^'!'] EXP CLOSE ALL_WP;
  let ELSE_EXP      = ALL_WP OPEN (IGN_WP "else" IGN_WP | '^') CLOSE ALL_WP;

  let COMMENT_EXP   = ALL_WP OPEN '!' EXP CLOSE ALL_WP;

  // then rules
  PASS_THROUGH      => |lexer:&mut HandleBarsLexer<R>| { Some( TokRaw( lexer.yystr() ) ) }

  SIMPLE_EXP        => |lexer:&mut HandleBarsLexer<R>| { Some( TokSimpleExp(     lexer.yystr() ) ) }
  NO_ESC_EXP        => |lexer:&mut HandleBarsLexer<R>| { Some( TokNoEscapeExp(   lexer.yystr() ) ) }
  PARTIAL_EXP       => |lexer:&mut HandleBarsLexer<R>| { Some( TokPartialExp(    lexer.yystr() ) ) }
  END_EXP           => |lexer:&mut HandleBarsLexer<R>| { Some( TokBlockEndExp(   lexer.yystr() ) ) }
  BLOCK_EXP         => |lexer:&mut HandleBarsLexer<R>| { Some( TokBlockExp(      lexer.yystr(), false ) ) }
  BLOCK_INV_EXP     => |lexer:&mut HandleBarsLexer<R>| { Some( TokBlockExp(      lexer.yystr(), true  ) ) }
  ELSE_EXP          => |lexer:&mut HandleBarsLexer<R>| { Some( TokBlockElseCond( lexer.yystr() ) ) }

  COMMENT_EXP       => |lexer:&mut HandleBarsLexer<R>| { Some( TokCommentExp(    lexer.yystr() ) ) }

  PASS_ESC          => |lexer:&mut HandleBarsLexer<R>| { Some( TokRaw( lexer.yystr()    ) ) }
  ESCAPED_EXP       => |    _:&mut HandleBarsLexer<R>| { Some( TokRaw( "{".to_string()  ) ) }
  ESCAPED_ESC       => |    _:&mut HandleBarsLexer<R>| { Some( TokRaw( "\\".to_string() ) ) }
  ESCAPED_SKIP      => |lexer:&mut HandleBarsLexer<R>| { Some( TokRaw( lexer.yystr()    ) ) }
}

rustlex! HBExpressionLexer {
  token HBToken;
  property in_options:bool = false;
  property in_params:bool = false;


  let NEW_LINE     =  '\r'?'\n';
  let IGN_WP        = [' ''\t']*;
  let ALL_WP        = (NEW_LINE | IGN_WP)+;

  let NO_WP       = '~';
  let START       = "{{" ['{''#''/''>''^''&']?;
  let START_NO_WP = "{{" '{'? NO_WP ['#''/''>''^''&']?;
  let END         =  '}'? "}}";

  let COMMENT_START       = "{{!";
  let COMMENT_START_NO_WP = "{{~!";
  let COMMENT_CONTENT     = (([^'}''~']|'}' [^'}']|'~' '}' [^'}'])?)*;

  let STRING_START = '"';
  let STRING_CTNT  = ("\\\"" | [^'"'])*; // either escaped quote or not quote
  let STRING_END   = ['"'];


  let IDENTIFIER = '@'? [^'!''"''#''%''&''\'''('')''*''+'',''.''/'';''<''=''>''@''[''\\'']''^''`''{''|''}''~'' ''\t']+;
  let BRACKET_ID_START = '[';
  let BRACKET_ID_END   = ']';
  let BRACKETED_ID     = [^']']+;
  let ACCESSOR_SEP     = ['.''/'];
  let ACCESSOR_END     = [' ''\t']+;

  let THIS             = "this" | ".";
  let PARENT_ALIAS     = "..";

  let PARAMS_SEP       = [' ''\t']+;

  let OPTION_NAME      = IDENTIFIER "=";

  INITIAL {
    NO_WP       => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.FORCE_END(); Some( TokNoWhiteSpaceAfter ) }

    START       => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.ACCESSOR(); None }
    START_NO_WP => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.ACCESSOR(); Some(TokNoWhiteSpaceBefore) }
    ALL_WP      => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { Some( TokLeadingWhiteSpace( lexer.yystr() ) ) }

    COMMENT_START       => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.COMMENT(); None }
    COMMENT_START_NO_WP => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.COMMENT(); Some(TokNoWhiteSpaceBefore) }

    END         => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }
  }

  ACCESSOR {
    IDENTIFIER       => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( lexer.yystr() ) ) }
    BRACKET_ID_START => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.ID_ANY(); None }

    STRING_START => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.STRING_PARAM(); None } // for parameters only

    THIS         => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( ".".to_string()  ) ) }
    PARENT_ALIAS => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( "..".to_string() ) ) }

    NO_WP       => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.FORCE_END(); Some( TokNoWhiteSpaceAfter ) }
    END         => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }
  }

  PROPERTY_PATH {
    ACCESSOR_SEP => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.ACCESSOR(); None }
    ACCESSOR_END => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> {
      if lexer.in_options  { lexer.OPTIONS() } else { lexer.PARAMS() };
      if lexer.in_params {
        Some( TokParamSep )
      } else {
        lexer.in_params = true;
        Some( TokParamStart )
      }
    }

    // common ending
    NO_WP        => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.FORCE_END(); Some( TokNoWhiteSpaceAfter ) }
    END          => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }
  }

  ID_ANY {
    BRACKETED_ID   => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { Some( TokPathEntry( lexer.yystr() ) ) }
    BRACKET_ID_END => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); None }
  }

  PARAMS {
    PARAMS_SEP   => |    _:&mut HBExpressionLexer<R>| -> Option<HBToken> { Some( TokParamSep ) }
    IDENTIFIER   => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( lexer.yystr() ) ) }
    STRING_START => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.STRING_PARAM(); None }

    THIS         => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( ".".to_string()  ) ) }
    PARENT_ALIAS => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( "..".to_string() ) ) }

    // end of parameters
    OPTION_NAME  => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> {
      lexer.in_options = true;
      lexer.OPTION_VALUE();
      Some( TokOption( lexer.yystr().trim_right_matches('=').to_string() ) )
    }

    // common expression ending
    NO_WP        => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.FORCE_END(); Some( TokNoWhiteSpaceAfter ) }
    END          => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }

  }

  STRING_PARAM {
    STRING_CTNT => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { Some( TokStringParam( lexer.yystr() ) ) }
    STRING_END  => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { if lexer.in_options  { lexer.OPTIONS() } else { lexer.PARAMS() }; None }
  }

  OPTION_VALUE {
    // all of these have conditional ending with in_params
    IDENTIFIER       => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( lexer.yystr() ) ) }
    BRACKET_ID_START => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.ID_ANY(); None }
    STRING_START     => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.STRING_PARAM(); None }

    THIS         => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( ".".to_string()  ) ) }
    PARENT_ALIAS => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.PROPERTY_PATH(); Some( TokPathEntry( "..".to_string() ) ) }

    // ok, pure option parsing for now
  }

  OPTIONS {
    OPTION_NAME  => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> {  lexer.OPTION_VALUE(); Some( TokOption( lexer.yystr().trim_right_matches('=').to_string() ) ) }
    PARAMS_SEP   => |    _:&mut HBExpressionLexer<R>| -> Option<HBToken> { None }

    // common expression ending
    NO_WP        => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.FORCE_END(); Some( TokNoWhiteSpaceAfter ) }
    END          => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }
  }

  COMMENT {
    END             => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }
    NO_WP           => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.FORCE_END(); Some( TokNoWhiteSpaceAfter ) }

    COMMENT_CONTENT => |    _:&mut HBExpressionLexer<R>| -> Option<HBToken> { None }
  }

  FORCE_END {
    END => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { lexer.TRAILING_WP(); None }
  }

  TRAILING_WP {
    ALL_WP => |lexer:&mut HBExpressionLexer<R>| -> Option<HBToken> { Some( TokTrailingWhiteSpace( lexer.yystr() ) ) }
  }



}

#[derive(Debug)]
pub enum HBValHolder {
  String(String),
  Path(Vec<String>),
  Literal(Json, String),
}

#[derive(Debug)]
pub struct RenderOptions {
  pub escape: bool,
  pub inverse: bool,
  pub indent: Option<String>,
  pub no_leading_whitespace: bool,
  pub no_trailing_whitespace: bool,
}

#[derive(Debug)]
pub struct HBExpression {
  pub base: Vec<String>,
  pub params: Vec<HBValHolder>,
  pub options: Vec<(String, HBValHolder)>,
  pub render_options: RenderOptions,
  pub block: Option<Box<Entries>>,
  pub else_block: Option<Box<Entries>>,
}

impl HBExpression {
  pub fn path(&self) -> String {
    let mut r = String::new();
    self.base.iter().take(self.base.len() - 1).fold(&mut r, |mut a, i| {a.push_str(&i); a.push('.'); a});
    self.base.last().map(|i| r.push_str(i));
    r
  }
}

type HBExpressionParsing = (Option<String>, HBExpression, Option<String>);

#[derive(Debug)]
pub enum HBEntry {
  Raw(String),
  Eval(HBExpression),
  Partial(HBExpression),
}

impl HBEntry {
  fn is_partial(&self) -> bool {
    match self {
      &HBEntry::Partial(_) => true,
      _ => false
    }
  }
}

pub type Entries = Vec<Box<HBEntry>>;
pub type ParseResult = Result<Template, (ParseError, Option<String>)>;



use std::io;
use super::{HBData, HBEvalResult, EvalContext, eval};


///
/// Internal representation of a handlebars template used for
/// later expansion.
///
/// Provides API shortcuts, parsing and expanding are internally
/// processed by [`::rumblebars::parse()`](fn.parse.html) and
/// [`::rumblebars::eval()`](fn.eval.html).
///
/// (see crate docs)

pub struct Template {
  pub entries: Entries
}

impl Template {
  pub fn new(template: &str) -> ParseResult {
    parse(template)
  }

  pub fn eval_to_string(&self, data: &HBData) -> Option<String> {
    let mut buf = Vec::new();
    self.eval(data, &mut buf, &Default::default()).ok().and_then(|_| String::from_utf8(buf).ok())
  }

  pub fn eval(&self, data: &HBData, out: &mut io::Write, eval_context: &EvalContext)  -> HBEvalResult {
    eval(&self, data, out, eval_context)
  }
}

/// only used internaly
impl ::std::default::Default for Template {
  fn default() -> Template {
    Template { entries: ::std::default::Default::default() }
  }
}

/// for `"{{hello}}".parse()` expressions
impl ::std::str::FromStr for Template {
  type Err = (ParseError, Option<String>);

  fn from_str(s: &str) -> Result<Self, <Self as ::std::str::FromStr>::Err> {
    parse(s)
  }
}

#[derive(Debug,Clone,Copy)]
pub enum ParseError {
  UnkownError, // unknown as ‘still not diagnosed’ case, not ’your grandma's TV is set on fire’ case
  InvalidExpression,
  UnmatchedBlock,
  UnexpectedBlockClose,
}

lazy_static! {
  static ref INDENT_MATCH: Regex = Regex::new("([:blank:]*)$").unwrap();
  static ref END_WP_TRIMMER: Regex = Regex::new("(\r?\n)[:blank:]*(\\{\\{~?[#!/](?:\\}?[^}])*\\}\\})[:blank:]*(:?\r?\n)?\\z").unwrap();
  static ref PARTIAL_END_WP_TRIMMER: Regex = Regex::new("(\r?\n[:blank:]*)(\\{\\{~?>(?:\\}?[^}])*\\}\\})[:blank:]*(:?\r?\n)?\\z").unwrap();

  static ref TRIM_LEAD_SPACE_MATCHER: Regex = Regex::new("((?:[:blank:]|\r?\n)*)(\r?\n)[:blank:]*$").unwrap();
  static ref TRIM_TRAIL_SPACE_MATCHER: Regex = Regex::new("^([:blank:]*\r?\n)(.*)").unwrap();
}

fn parse_hb_expression(exp: &str) -> Result<HBExpressionParsing, (ParseError, Option<String>)> {
  let mut lexer = HBExpressionLexer::new(BufReader::new(exp.as_bytes()));
  let mut render_options = RenderOptions {
    escape: true,
    indent: None,
    no_leading_whitespace: false,
    no_trailing_whitespace: false,
    inverse: false
  };
  let mut leading_whitespace = None;
  let mut trailing_whitespace = None;
  let mut path = vec![];
  let mut params = vec![];
  let mut options = vec![];


  while let Some(tok) = lexer.next() {
    match tok {
      TokLeadingWhiteSpace(s) => {
        render_options.indent = INDENT_MATCH.captures(&s).and_then(|s| s.at(1) ).map(|s| s.to_string());
        leading_whitespace = Some(s);
      },

      TokNoWhiteSpaceBefore    => { render_options.no_leading_whitespace = true },
      TokNoWhiteSpaceAfter     => { render_options.no_trailing_whitespace = true },
      TokTrailingWhiteSpace(s) => { trailing_whitespace = Some(s) },
      TokPathEntry(path_comp)  => { path.push(path_comp) },
      TokStringParam(path_comp)  => { path.push(path_comp) },

      TokParamStart => {
        let mut param_path = vec![];
        while let Some(tok) = lexer.next() {
          match tok {
            TokPathEntry(path_comp) => { param_path.push(path_comp) },
            TokStringParam(s) => { params.push(HBValHolder::String(s)) },
            TokParamSep => {
              if param_path.len() > 0 {
                params.push(HBValHolder::Path(param_path));
                param_path = vec![];
              }
            },
            // options starts here
            TokOption(opt) => {
              let option_name = opt;
              let mut opt_path = vec![];
              let mut opt_val  = None;

              // we have an option, get its value and following options
              while let Some(tok) = lexer.next() {
                match tok {
                  TokPathEntry(s) => {
                    opt_path.push(s);
                  },
                  TokStringParam(s) => {
                    opt_val = Some(s);
                    break;
                  },
                  TokNoWhiteSpaceAfter => { render_options.no_trailing_whitespace = true },
                  TokTrailingWhiteSpace(s) => { trailing_whitespace = Some(s) },
                  _ => { break }
                }
              }

              options.push((option_name, if let Some(val) = opt_val { HBValHolder::String(val) } else { HBValHolder::Path(opt_path) }));

            },
            TokNoWhiteSpaceAfter => { render_options.no_trailing_whitespace = true },
            TokTrailingWhiteSpace(s) => { trailing_whitespace = Some(s) },
            _ => { break; }
          }
        }
        let literal_param = match param_path.first() {
          Some(s) if param_path.len() == 1 => {
            if let Ok(j) = Json::from_str(s) {
              Some(HBValHolder::Literal(j, s.clone()))
            } else {
              None
            }
          },
          Some(_) | None => None,
        };

        if let Some(p) = literal_param {
          params.push(p);
        } else if param_path.len() > 0 {
          params.push(HBValHolder::Path(param_path));
        }
      },
      _ => { break },
    }
  }


  return  Ok((
    leading_whitespace,
    HBExpression {
      base: path,
      params: params,
      options: options,
      render_options: render_options,
      block: None,
      else_block: None
    },
    trailing_whitespace
  ))
}

// after handling parsed token, handle result and leading/trailing whitespace
#[derive(Debug)]
enum Unit {
  AppendRaw(Box<HBEntry>),
  Append(Option<String>, Box<HBEntry>, Option<String>),
  AppendAutoTrim(Option<String>, Box<HBEntry>, Option<String>),
  Shift(Option<String>,  Box<HBEntry>, bool, Option<String>),
  Reduce(Option<String>, Box<HBEntry>, Option<String>),
  TrimOnly(Option<String>, Box<HBEntry>, Option<String>),
}

// append entry to stack but if entry is raw data, append it to last raw entry
fn append_entry(stack: &mut Vec<(Box<Entries>, bool)>, e: Box<HBEntry>) {
  let may_push_entry = match (stack.last_mut(), &*e) {
    (Some(&mut(ref mut block, _)), &HBEntry::Raw(ref s)) => {
      if let Some(ref mut boxed) = (***block).last_mut() {
        if let HBEntry::Raw(ref mut existing) = ***boxed {
          existing.push_str(&s);
          false
        } else {
          true
        }
      } else {
        true
      }
    },
    _ => true,
  };

  if may_push_entry {
    match stack.last_mut() {
      Some(&mut (ref mut block, _)) => {
        block.push(e)
      },
      _ => ()
    }
  }
}

/// Parses a handlebars template.
///
///
/// # Failures
///
/// Handlebars syntax does not have much corner cases, so hopefully
/// you won't get much cryptic cases. The most painfull ones are
/// unmatched blocks opening/closing, this class of errors comes with
/// a short description.
///
/// # Examples
///
/// Provided that you have continous integration with tests on your code,
/// unwraping is safe for static inline templates.
///
/// ```
/// ::rumblebars::parse("{{hello}}").unwrap();
/// ```
///
/// Otherwise check for errors.
///
/// ```
/// assert!(::rumblebars::parse("{{#hello}}{{/end}}").is_err());
/// ```

pub fn parse(template: &str) -> ParseResult {
  // trimming template handling with a regex, as rustlex does not emit tokens on input end,
  // but it's very (very) convenient for this case

  let trimmed = END_WP_TRIMMER.replace_all(&template,"$1$2");
  let trimmed = PARTIAL_END_WP_TRIMMER.replace_all(&trimmed,"$1$2");

  let lexer = HandleBarsLexer::new(BufReader::new(trimmed.as_bytes()));

  // parse stack entry tuple: (template, is_else_block)
  let mut stack = vec![(Box::new(vec![]) , false)];


  let mut previous_trail_whitespace: Option<(String, bool)> = None;
  let mut first = true;

  for tok in lexer {
    // handle each token specifities and distribute them to generic shift/reduce handlings
    let token_result = match tok {
      TokRaw(s) => {
        Unit::AppendRaw(Box::new(HBEntry::Raw(s)))
      },
      TokSimpleExp(ref exp) => {
        if let Ok((lead_wp, hb, trail_wp)) = parse_hb_expression(&exp) {
          Unit::Append(lead_wp, Box::new(HBEntry::Eval(hb)), trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      },
      TokCommentExp(ref exp) => {
        if let Ok((lead_wp, hb, trail_wp)) = parse_hb_expression(&exp) {
          Unit::TrimOnly(lead_wp, Box::new(HBEntry::Eval(hb)), trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      },
      TokNoEscapeExp(ref exp) => {
        if let Ok((lead_wp, mut hb, trail_wp)) = parse_hb_expression(&exp) {
          hb.render_options.escape = false;
          Unit::Append(lead_wp, Box::new(HBEntry::Eval(hb)), trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      },
      TokPartialExp(ref exp) => {
        if let Ok((lead_wp, hb, trail_wp)) = parse_hb_expression(&exp) {
          Unit::AppendAutoTrim(lead_wp, Box::new(HBEntry::Partial(hb)), trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      },
      TokBlockExp(ref exp, inverse) => {
        if let Ok((lead_wp, mut hb, trail_wp)) = parse_hb_expression(&exp) {
          hb.render_options.inverse = inverse;
          Unit::Shift(lead_wp, Box::new(HBEntry::Eval(hb)), false, trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      },
      TokBlockElseCond(ref exp) => {
        if let Ok((lead_wp, hb, trail_wp)) = parse_hb_expression(&exp) {
          Unit::Shift(lead_wp, Box::new(HBEntry::Eval(hb)), true, trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      },
      TokBlockEndExp(ref exp) => {
        if let Ok((lead_wp, hb, trail_wp)) = parse_hb_expression(&exp) {
          Unit::Reduce(lead_wp, Box::new(HBEntry::Eval(hb)), trail_wp)
        } else {
          return Result::Err((ParseError::InvalidExpression, Some(format!("Could not parse {:?}", exp))));
        }
      }
    };

    match token_result {
      // direct append without trimming
      Unit::AppendRaw(entry) => {
        match previous_trail_whitespace {
          Some((s, true)) => append_entry(&mut stack, Box::new(HBEntry::Raw(s))),
          _ => ()
        };

        previous_trail_whitespace = None;
        append_entry(&mut stack, entry);
      },
      // direct append with explicit trimming
      Unit::Append(lead_wp, entry, trail_wp) => {
        let (remove_lead_wp, remove_trail_wp) = match *entry {
          HBEntry::Eval(ref exp) => (exp.render_options.no_leading_whitespace, exp.render_options.no_trailing_whitespace),
          HBEntry::Partial(ref exp) => (exp.render_options.no_leading_whitespace, exp.render_options.no_trailing_whitespace),
          _ => (false, false),
        };

        // if we have previous trail, it's our current leading, so skip if we trim lead
        match (previous_trail_whitespace, remove_lead_wp) {
          (Some((s, true)), false) => append_entry(&mut stack, Box::new(HBEntry::Raw(s))),
          _ => ()
        };

        previous_trail_whitespace = trail_wp.map(|s| (s, !remove_trail_wp) );

        match (lead_wp, remove_lead_wp) {
          (Some(space), false) => append_entry(&mut stack, Box::new(HBEntry::Raw(space))),
          _ => ()
        }

        append_entry(&mut stack, entry);

      },
      // shift or reduce with auto trim
      autotrimable @ Unit::Shift(..) |
      autotrimable @ Unit::Reduce(..) |
      autotrimable @ Unit::TrimOnly(..) |
      autotrimable @ Unit::AppendAutoTrim(..) => {
        let (shift, reduce, append, lead_wp, entry, is_else, trail_wp) = match autotrimable {
          Unit::Shift(lead_wp, entry, is_else, trail_wp)   => (true, false, false, lead_wp, entry, is_else, trail_wp),
          Unit::Reduce(lead_wp, entry, trail_wp)           => (false, true, false,  lead_wp, entry, false, trail_wp),
          Unit::AppendAutoTrim(lead_wp, entry, trail_wp)   => (false, false, true,  lead_wp, entry, false, trail_wp),
          Unit::TrimOnly(lead_wp, entry, trail_wp)         => (false, false, false,  lead_wp, entry, false, trail_wp),
          _ => panic!("rustc did compile some weird case"),
        };

        // extract whitespace options
        let (remove_lead_wp, remove_trail_wp) = match *entry {
          HBEntry::Eval(ref exp) => (exp.render_options.no_leading_whitespace, exp.render_options.no_trailing_whitespace),
          HBEntry::Partial(ref exp) => (exp.render_options.no_leading_whitespace, exp.render_options.no_trailing_whitespace),
          _ => (false, false),
        };

        //
        // calculate autotrimming
        //   note that end of input trailing space is handled by a prelude replace regex
        //

        // we use owned leading whitespace, or use previous elligible trailing whitespace
        // or fallback to a default
        let lead_space_with_fallbacks = lead_wp.clone()
          .map(|wp| (wp, true))
          .or(previous_trail_whitespace.clone().map(|(wp, can_be_used)| (wp, can_be_used)))
          .or(Some(("".to_string(), false)));

        let (trimmed, trail_match, trail_keep) = match (lead_space_with_fallbacks, trail_wp.clone()) {
          (Some((ref lead_space, owned_space)), Some(ref trail_space)) => {

            // lead space : split in kept wp and new line that replaces trimmed space
            //   fallback to neutral values if first encountered exp is trimmable
            let lead_matches  = if first {
              // first token, make lead space elligible to trimming, but empty
              Some((Some(""), Some("")))
            } else {
              // check against auto trim leading space rules
              TRIM_LEAD_SPACE_MATCHER.captures(&lead_space).and_then(|s| Some((s.at(1), s.at(2))))
            };

            // check trailing whitespace against auto trim trailing space rules
            let trail_matches = TRIM_TRAIL_SPACE_MATCHER.captures(&trail_space)
              .and_then(|s| Some((s.at(1).map(|p| p.to_string()), s.at(2).map(|p| p.to_string()))));

            // check matches, is both are ok, go on trimming
            match (lead_matches, trail_matches) {
              (Some((Some(lead_keep), Some(lead_nl_match))), Some((trail_m, trail_k))) => {
                let mut to_insert = String::new();
                if owned_space && lead_keep.len() > 0 {
                  // only insert owned whitespace
                  to_insert.push_str(lead_keep);
                }
                if owned_space && lead_nl_match.len() > 0 {
                  // insert autotrim newline when whitespace is owned and expression  candidate to autriming
                  to_insert.push_str(lead_nl_match);
                }

                if to_insert.len() > 0 && !remove_lead_wp {
                  append_entry(&mut stack, Box::new(HBEntry::Raw(to_insert)));
                }

                (true, trail_m, trail_k)
              }
              _ => (false, None, trail_wp),
            }
          },
          _ => (false, None, trail_wp),
        };


        // if there is not autotrim nor explicit trimming, push leading whitespace, that might come from previous trailing
        let usable_previous = previous_trail_whitespace.and_then(|(s, can_use)| if can_use { Some(s) } else { None });

        if let (false, false, Some(space)) = (trimmed, remove_lead_wp, lead_wp.or(usable_previous)) {
          append_entry(&mut stack, Box::new(HBEntry::Raw(space)));
        }

        // keep elligible trailing whitespace for next expression auto trimming check
        previous_trail_whitespace = trail_keep.clone().and_then(|k| {if k == "" { None } else { Some((k, !remove_trail_wp)) }} ).or(trail_match.clone().map(|s| (s, false)));

        if shift || append {
          // first, just handle partial trimming specific handling for indentation
          if trimmed && entry.is_partial() {
            match *entry {
              HBEntry::Partial(HBExpression {render_options: RenderOptions {indent: Some(ref s), ..}, ..}) => append_entry(&mut stack, Box::new(HBEntry::Raw(s.clone()))),
              _ => ()
            }
          }

          // append, push entry into current collector
          if ! is_else {
            append_entry(&mut stack, entry);
          }

          if shift {
            // compilation shifting : entry was pushed, and a new collector is inserted
            stack.push((Box::new(vec![]), is_else));
          }

        } else if reduce {
          // reducing : inspect stack and reduce last elligible token collectors into their parent
          // (remove entries from stack and attach them to their parent )

          // check if it's a signle block or a block/else reduction
          let has_else = match stack.last() { Some(&(_, true)) => true, _ => false };

          // pop reduced content from stack
          let pop = if has_else {
            (stack.pop(), stack.pop())
          } else {
            (None, stack.pop())
          };

          // attach content to parent

          if let Some(&mut (ref mut parents, _)) = stack.last_mut() {
            if let HBEntry::Eval(ref hb) = *entry {
              match (***parents).last_mut() {
                Some(ref mut boxed_parent) => {
                  if let HBEntry::Eval(ref mut parent) = ***boxed_parent {
                    if parent.base == hb.base {
                      match pop {
                        (some_else, Some((block, _))) => {
                          parent.block = Some(block);
                          if let Some((else_block, _)) = some_else {
                            parent.else_block = Some(else_block);
                          }
                        },
                        _ => panic!("(some_else, Some((block, _))) pattern should always be matched — parse.rs#parse")
                      }

                    } else {
                      return Err((
                        ParseError::UnmatchedBlock,
                        Some(format!("‘{}’ does not match ‘{}’", hb.path(), parent.path()))
                      ));
                    }
                  } else {
                    return Err((
                      ParseError::UnexpectedBlockClose,
                      Some(format!("‘{}’ does not close any block", hb.path()))
                    ));
                  }
                }
                _ => {
                  return Err((
                    ParseError::UnexpectedBlockClose,
                    Some(format!("‘{}’ does not close any block", hb.path()))
                  ));
                }
              }
            } else {
              panic!("Should not reach: there's a bug in handelbars template@ parser, we're doing a block reduce on invalid parsing state");
            }
          } else {
            panic!("Should not reach: there's a bug in handelbars template@ parser, we're doing a block reduce on invalid parsing state");
          }

        }
      },
    }

    // not first token anymore
    first = false;
  }

  match previous_trail_whitespace {
    Some((ref s, true)) =>  {
      append_entry(&mut stack, Box::new(HBEntry::Raw(s.clone())))
    },
    _ => ()
  };

  if stack.len() > 0 {
    Result::Ok(Template { entries: *stack.remove(0).0 })
  } else {
    Result::Err((ParseError::UnkownError, None))
  }
}

#[cfg(test)]
mod tests {
  use std::default::Default;
  use super::{parse, parse_hb_expression, HBEntry, HBExpression, HBValHolder, Template};

  #[test]
  fn hb_simple() {
    assert!(match parse_hb_expression("{{i}}") {
      Ok(_)  => true,
      Err(_) => false,
    })
  }

  #[test]
  fn hb_simple_base() {
    match parse_hb_expression("{{i}}") {
      Ok((_, ok, _))  => assert_eq!(ok.base, vec!["i"]),
      Err(_)  => (),
    }
  }

  #[test]
  fn hb_simple_base_path() {
    match parse_hb_expression("{{i.j}}") {
      Ok((_, ok, _))  => assert_eq!(ok.base, vec!["i", "j"]),
      Err(_)  => (),
    }
  }

  #[test]
  fn hb_simple_base_esc_path() {
    match parse_hb_expression("{{[i]}}") {
      Ok((_, ok, _))  => assert_eq!(ok.base, vec!["i"]),
      Err(_)  => (),
    }
  }

  #[test]
  fn hb_simple_this_path() {
    match parse_hb_expression("{{.}}") {
      Ok((_, ok, _))  => assert_eq!(ok.base, vec!["."]),
      Err(_)  => (),
    }
  }

  #[test]
  fn hb_this_path() {
    match parse_hb_expression("{{./p}}") {
      Ok((_, ok, _))  => assert_eq!(ok.base, vec![".", "p"]),
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_string_param() {
    match parse_hb_expression(r##"{{p "string"}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["p"]);
        assert_eq!(match params.get(0).unwrap() { &HBValHolder::String(ref s) => s.clone(), _ => "".to_string()}, "string".to_string());
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_prop_path_param() {
    match parse_hb_expression(r##"{{p some.path}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["p"]);
        assert_eq!(match params.get(0).unwrap() { &HBValHolder::Path(ref p) => p.clone(), _ => vec![]}, vec!["some", "path"]);
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_2_params() {
    match parse_hb_expression(r##"{{p some path}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["p"]);
        assert_eq!(match params.get(0).unwrap() { &HBValHolder::Path(ref p) => p.clone(), _ => vec![]}, vec!["some"]);
        assert_eq!(match params.get(1).unwrap() { &HBValHolder::Path(ref p) => p.clone(), _ => vec![]}, vec!["path"]);
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_3_params() {
    match parse_hb_expression(r##"{{p some.path "with_string" yep}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["p"]);
        assert_eq!(match params.get(0).unwrap() { &HBValHolder::Path(ref p) => p.clone(), _ => vec![]}, vec!["some", "path"]);
        assert_eq!(match params.get(1).unwrap() { &HBValHolder::String(ref s) => s.clone(), _ => "".to_string()}, "with_string".to_string());
        assert_eq!(match params.get(2).unwrap() { &HBValHolder::Path(ref p) => p.clone(), _ => vec![]}, vec!["yep"]);
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_full_feat_param() {
    match parse_hb_expression(r##"{{t "… param1" well.[that my baby].[1] ~}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["t"]);
        assert_eq!(match params.get(0).unwrap() { &HBValHolder::String(ref s) => s.clone(), _ => "".to_string()}, "… param1".to_string());
        assert_eq!(match params.get(1).unwrap() { &HBValHolder::Path(ref p) => p.clone(), _ => vec![]}, vec!["well", "that my baby", "1"]);
        assert!(render_options.no_trailing_whitespace);
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_option() {
    match parse_hb_expression(r##"{{t opt=u ~}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["t"]);
        assert_eq!(("opt".to_string(), vec!["u".to_string()]), match options.get(0).unwrap() {
          &(ref o, HBValHolder::Path(ref p)) => (o.clone(), p.clone()),
          _ => ("".to_string(), vec![]),
        });
        assert!(render_options.no_trailing_whitespace);
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_mutli_options() {
    match parse_hb_expression(r##"{{t opt=u opt2="v" ~}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["t"]);
        assert_eq!(("opt".to_string(), vec!["u".to_string()]), match options.get(0).unwrap() {
          &(ref o, HBValHolder::Path(ref p)) => (o.clone(), p.clone()),
          _ => ("".to_string(), vec![]),
        });
        assert_eq!(("opt2".to_string(), "v".to_string()), match options.get(1).unwrap() {
          &(ref o, HBValHolder::String(ref s)) => (o.clone(), s.clone()),
          _ => ("".to_string(), "".to_string()),
        });
        assert!(render_options.no_trailing_whitespace);
      },
      Err(_)  => (),
    }
  }

  #[allow(unused_variables)]
  #[test]
  fn hb_param_options() {
    match parse_hb_expression(r##"{{t o.[t}+=] opt="v" ~}}"##) {
      Ok((_, HBExpression{ref base, ref params, ref options, ref render_options, ref block, ref else_block}, _))  => {
        assert_eq!(base, &vec!["t"]);
        assert_eq!(vec!["o", "t}+="], match params.get(0).unwrap() {
          &HBValHolder::Path(ref p) => p.clone(), _ => vec![]
        });
        assert_eq!(("opt".to_string(), "v".to_string()), match options.get(0).unwrap() {
          &(ref o, HBValHolder::String(ref s)) => (o.clone(), s.clone()),
          _ => ("".to_string(), "".to_string()),
        });
        assert!(render_options.no_trailing_whitespace);
      },
      Err(_)  => (),
    }
  }

  #[test]
  fn from_str() {
    let template = "t {{u}} v".parse::<Template>();
    assert!(template.is_ok())
  }

  #[test]
  fn parse_raw() {
    let p = parse("tada").unwrap_or(Default::default());
    assert_eq!("tada", match p.entries.get(0) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Raw(ref s) => &s[..],
          _ => "",
        }
      }
      _ => "",
    });
  }

  #[test]
  fn parse_exp() {
    let p = parse("{{tada}}").unwrap_or(Default::default());
    assert_eq!("tada", match p.entries.get(0) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Eval(HBExpression {ref base, ..}) => &base.iter().next().unwrap()[..],
          _ => "",
        }
      }
      _ => "",
    });
  }

  #[allow(unused_variables)]
  #[test]
  fn parse_else_block() {
    let p = parse("{{#tada}}i{{else}}o{{/tada}}").unwrap_or(Default::default());;
    assert_eq!(true, match p.entries.get(0) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Eval(HBExpression {ref base, ref params, ref options, ref render_options, ref block, ref else_block}) => match (block, else_block) { (&Some(_), &Some(_)) => true, _ => false },
          _ => false,
        }
      },
      _ => false,
    });
  }


  #[test]
  fn parse_exp_entangled() {
    let p = parse("tidi {{tada}} todo {{tudu}} bar").unwrap_or(Default::default());
    assert_eq!("tidi ", match p.entries.get(0) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Raw(ref s) => &s[..],
          _ => "",
        }
      }
      _ => "",
    });
    assert_eq!("tada", match p.entries.get(1) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Eval(HBExpression {ref base, ..}) => &base.iter().next().unwrap()[..],
          _ => "",
        }
      }
      _ => "",
    });
    assert_eq!(" todo ", match p.entries.get(2) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Raw(ref s) => &s[..],
          _ => "",
        }
      }
      _ => "",
    });
    assert_eq!("tudu", match p.entries.get(3) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Eval(HBExpression {ref base, ..}) => &base.iter().next().unwrap()[..],
          _ => "",
        }
      }
      _ => "",
    });
    assert_eq!(" bar", match p.entries.get(4) {
      Some(& ref boxed_entry) => {
        match **boxed_entry {
          HBEntry::Raw(ref s) => &s[..],
          _ => "",
        }
      }
      _ => "",
    });
  }

}

#[cfg(test)]
#[cfg(feature = "nightly")]
mod bench {

  use super::parse_hb_expression;
  use test::Bencher;

  #[bench]
  fn parse_simple_hb_exp(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression("{{i}}").ok();
    })
  }


  #[bench]
  fn parse_hb_exp_1(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression("{{i.j}}").ok();
    })
  }

  #[bench]
  fn parse_hb_exp_2(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression("{{[i]}}").ok();
    })
  }

  #[bench]
  fn parse_hb_exp_3(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression("{{.}}").ok();
    })
  }

  #[bench]
  fn parse_hb_exp_4(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression("{{./p}}").ok();
    })
  }

  #[bench]
  fn parse_hb_exp_5(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{p "string"}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_6(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{p some.path}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_7(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{p some path}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_8(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{p some.path "with_string" yep}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_9(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{t "… param1" well.[that my baby].[1] ~}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_10(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{t opt=u ~}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_11(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{t opt=u opt2="v" ~}}"##).ok();
    })
  }

  #[bench]
  fn parse_hb_exp_12(b: &mut Bencher) {
    b.iter(|| {
      parse_hb_expression(r##"{{t o.[t}+=] opt="v" ~}}"##).ok();
    })
  }
}
