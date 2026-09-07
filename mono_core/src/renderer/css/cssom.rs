use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::iter::Peekable;

use crate::renderer::css::token::{CssToken, CssTokenizer};

#[derive(Debug, Clone)]
pub struct CssParser {
    t: Peekable<CssTokenizer>,
}

impl CssParser {
    pub fn new(t: CssTokenizer) -> Self {
        Self { t: t.peekable() }
    }

    fn consume_ident(&mut self) -> String {
        let token = match self.t.next() {
            Some(t) => t,
            None => panic!("should have a token but got None"),
        };

        match token {
            CssToken::Ident(ref ident) => ident.to_string(),
            _ => {
                panic!("Parse error: {:?} is an unexpected token.", token);
            }
        }
    }

    fn consume_component_value(&mut self) -> ComponentValue {
        self.t
            .next()
            .expect("should have a token in consume_component_value")
    }

    fn consume_declaration(&mut self) -> Option<Declaration> {
        if self.t.peek().is_none() {
            return None;
        }

        let mut declaration = Declaration::new();
        declaration.property = self.consume_ident();
        match self.t.next() {
            Some(token) => match token {
                CssToken::Colon => {}
                _ => return None,
            },
            None => return None,
        }

        declaration.value = self.consume_component_value();

        Some(declaration)
    }

    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return declarations,
            };

            match token {
                CssToken::CloseCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::CloseCurly));
                    return declarations;
                }
                CssToken::SemiColon => {
                    assert_eq!(self.t.next(), Some(CssToken::SemiColon));
                }
                CssToken::Ident(ref _ident) => {
                    if let Some(declaration) = self.consume_declaration() {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    self.t.next();
                }
            }
        }
    }

    fn consume_selector(&mut self) -> Selector {
        let token = match self.t.next() {
            Some(t) => t,
            None => panic!("should have a token but got None"),
        };

        match token {
            CssToken::HashToken(value) => Selector::IdSelector(value[1..].to_string()),
            CssToken::Delim(delim) => {
                if delim == '.' {
                    return Selector::ClassSelector(self.consume_ident());
                }
                panic!("Parse error: {:?} is an unexpected token.", token);
            }
            CssToken::Ident(ident) => {
                // a:hoverのようなセレクタ(pseudo class: https://www.w3.org/TR/selectors-4/#pseudo-classes)は無視する
                if self.t.peek() == Some(&CssToken::Colon) {
                    while self.t.peek() != Some(&CssToken::OpenCurly) {
                        self.t.next();
                    }
                }

                Selector::TypeSelector(ident.to_string())
            }
            CssToken::AtKeyword(_keyword) => {
                // @から始まるルールは無視する
                while self.t.peek() != Some(&CssToken::OpenCurly) {
                    self.t.next();
                }

                Selector::UnknownSelector
            }
            _ => {
                self.t.next();

                Selector::UnknownSelector
            }
        }
    }

    fn consume_qualified_rule(&mut self) -> Option<QualifiedRule> {
        let mut rule = QualifiedRule::new();

        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return None,
            };

            match token {
                CssToken::OpenCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::OpenCurly));
                    rule.declarations = self.consume_list_of_declarations();
                    return Some(rule);
                }
                _ => {
                    rule.selector = self.consume_selector();
                }
            }
        }
    }

    fn consume_list_of_rules(&mut self) -> Vec<QualifiedRule> {
        let mut rules = Vec::new();

        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return rules,
            };
            match token {
                CssToken::AtKeyword(_keyword) => {
                    // @から始まるルールは無視する
                    let _rule = self.consume_qualified_rule();
                }
                _ => {
                    let rule = self.consume_qualified_rule();
                    match rule {
                        Some(r) => rules.push(r),
                        None => return rules,
                    }
                }
            }
        }
    }

    pub fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut sheet = StyleSheet::new();
        sheet.rules = self.consume_list_of_rules();

        sheet
    }
}

/// https://www.w3.org/TR/cssom/#the-cssstylesheet-interface
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSheet {
    /// https://www.w3.org/TR/cssom/#dom-cssstylesheet-cssrules
    pub rules: Vec<QualifiedRule>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
}

/// https://www.w3.org/TR/css-syntax-3/#qualified-rule
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRule {
    /// https://www.w3.org/TR/selectors-4/#typedef-selector-list
    pub selector: Selector,
    /// https://www.w3.org/TR/css-syntax-3/#parse-a-list-of-declarations
    pub declarations: Vec<Declaration>,
}

impl QualifiedRule {
    pub fn new() -> Self {
        Self {
            selector: Selector::TypeSelector("".to_string()),
            declarations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// https://www.w3.org/TR/selectors-4/#type-selectors
    TypeSelector(String),
    /// https://www.w3.org/TR/selectors-4/#class-html
    ClassSelector(String),
    /// https://www.w3.org/TR/selectors-4/#id-selectors
    IdSelector(String),
    /// パース中にエラーが起こった時に使用するセレクタ
    UnknownSelector,
}

/// https://www.w3.org/TR/css-syntax-3/#declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: ComponentValue,
}

impl Declaration {
    pub fn new() -> Self {
        Self {
            property: String::new(),
            value: ComponentValue::Ident(String::new()),
        }
    }
}

/// https://www.w3.org/TR/css-syntax-3/#component-value
/// cf. https://www.w3.org/TR/css-values-4/
pub type ComponentValue = CssToken;

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_empty() {
        let style = "".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        assert_eq!(cssom.rules.len(), 0);
    }

    #[test]
    fn test_one_rule() {
        let style = "p { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::new();
        rule.selector = Selector::TypeSelector("p".to_string());

        let mut declaration = Declaration {
            property: "color".to_string(),
            value: ComponentValue::Ident("red".to_string()),
        };
        declaration.property = "color".to_string();
        declaration.value = ComponentValue::Ident("red".to_string());

        rule.declarations = vec![declaration];

        let expected = [rule];

        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);

            i += 1;
        }
    }

    #[test]
    fn test_id_selector() {
        let style = "#id { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::new();
        rule.selector = Selector::IdSelector("id".to_string());

        let mut declaration = Declaration::new();
        declaration.property = "color".to_string();
        declaration.value = ComponentValue::Ident("red".to_string());

        rule.declarations = vec![declaration];

        let expected = [rule];

        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);

            i += 1;
        }
    }

    #[test]
    fn test_class_selector() {
        let style = ".class { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::new();
        rule.selector = Selector::ClassSelector("class".to_string());

        let mut declaration = Declaration::new();
        declaration.property = "color".to_string();
        declaration.value = ComponentValue::Ident("red".to_string());

        rule.declarations = vec![declaration];

        let expected = [rule];

        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);

            i += 1;
        }
    }

    #[test]
    fn test_multiple_rules() {
        let style = "p { content: \"Hey\"; } h1 { font-size: 40; color: blue; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule1 = QualifiedRule::new();
        rule1.selector = Selector::TypeSelector("p".to_string());

        let mut declaration1 = Declaration::new();
        declaration1.property = "content".to_string();
        declaration1.value = ComponentValue::StringToken("Hey".to_string());

        rule1.declarations = vec![declaration1];

        let mut rule2 = QualifiedRule::new();
        rule2.selector = Selector::TypeSelector("h1".to_string());

        let mut declaration2 = Declaration::new();
        declaration2.property = "font-size".to_string();
        declaration2.value = ComponentValue::Number(40.0);

        let mut declaration3 = Declaration::new();
        declaration3.property = "color".to_string();
        declaration3.value = ComponentValue::Ident("blue".to_string());

        rule2.declarations = vec![declaration2, declaration3];

        let expected = [rule1, rule2];
        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);

            i += 1;
        }
    }
}
