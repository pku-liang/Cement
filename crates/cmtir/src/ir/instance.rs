use std::fmt::{self, Debug};

use super::*;

/// An instance-def represents an instance of a module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SExpr)]
pub struct InstDef {
  #[pp(open)]
  pub name: String,
  #[pp(close)]
  pub module: String,
}

impl InstDef {
  pub fn new(name: String, module: String) -> Self {
    InstDef {
      name,
      module,
    }
  }

}

/// An instance-rule represents one rule call of an instance. The last segment
/// is the rule name.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstRule {
  pub path: Vec<String>,
  pub rule_name: String,
}

impl InstRule {
  /// Check if the instance rule is the self instance.
  pub fn is_self(&self) -> bool {
    self.path.is_empty() || self.path == vec!["self".to_string()]
  }
  /// Create a new instance rule with the given rule name.
  pub fn with_rule_name(self, rule_name: String) -> Self {
    InstRule {
      path: self.path,
      rule_name,
    }
  }
  /// Canonicalize the instance rule.
  pub fn canonicalize(&self) -> InstRule {
    InstRule {
      path: if self.path.is_empty() {
        vec!["self".to_string()]
      } else {
        self.path.clone()
      },
      rule_name: self.rule_name.clone(),
    }
  }
  /// Pre-extend the instance rule with the given segments.
  pub fn pre_extend(mut self, segments: &Vec<String>) -> InstRule {
    let mut segments = segments.clone();
    self.path.retain(|s| s != "self");
    segments.extend(self.path);
    InstRule {
      path: segments,
      rule_name: self.rule_name,
    }
    .canonicalize()
  }
  /// Convert the instance rule to string.
  pub fn to_string(&self) -> String {
    let mut result = "".to_string();
    result.push_str(&self.path.join("."));
    if !self.path.is_empty() {
      result.push('.');
    }
    result.push_str(&self.rule_name.clone());
    result
  }
  /// Convert the instance rule to string with the given separator.
  pub fn to_string_with(&self, sep: &str) -> String {
    let mut result = "".to_string();

    if !self.is_self() {
      result.push_str(&self.path.join(sep));
      result.push_str(sep);
    }
    result.push_str(&self.rule_name.clone());
    result
  }
}

impl Debug for InstRule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl Parse for InstRule {
  fn parse(p: &mut Parser) -> Result<Self, String> {
    let mut segments = vec![p.expect(Token::Keyword)?.to_string()];

    while p.peek_fn(|s, _| s == ".") {
      let _ = p.expect(Token::Punct);
      segments.push(p.expect(Token::Keyword)?.to_string());
    }

    let rule_name = segments.pop().unwrap();

    Ok(
      InstRule {
        path: segments,
        rule_name,
      }
      .canonicalize(),
    )
  }
}

impl Print for InstRule {
  fn print(&self, p: &mut Printer) {
    let canonical = self.canonicalize();
    let s = canonical.path.join(".");
    write!(p, "{}.{}", s, canonical.rule_name);
  }
}

impl InstRule {
  /// Create a new instance rule with the given segments.
  pub fn new(mut segments: Vec<String>) -> Self {
    let rule_name = segments.pop().unwrap();
    InstRule {
      path: segments,
      rule_name,
    }
  }
  /// Create a new instance rule from the given string.
  pub fn from_str(s: &str) -> Self {
    let mut segments: Vec<String> =
      s.split('.').map(|s| s.to_string()).collect();
    let rule_name = segments.pop().unwrap();
    InstRule {
      path: segments,
      rule_name,
    }
    .canonicalize()
  }
}

impl From<String> for InstRule {
  fn from(s: String) -> Self {
    InstRule::from_str(&s)
  }
}

impl ToString for InstRule {
  fn to_string(&self) -> String {
    let canonical = self.canonicalize();
    format!("{}.{}", canonical.path.join("."), canonical.rule_name)
  }
}
