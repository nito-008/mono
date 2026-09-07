use core::cell::RefCell;
use core::str::FromStr;

use alloc::rc::{Rc, Weak};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::constants::{
    CHAR_HEIGHT_WITH_PADDING, CHAR_WIDTH, CONTENT_AREA_WIDTH, WINDOW_PADDING, WINDOW_WIDTH,
};
use crate::display_item::DisplayItem;
use crate::renderer::css::cssom::{ComponentValue, Declaration, Selector, StyleSheet};
use crate::renderer::dom::node::{Node, NodeKind};
use crate::renderer::layout::computed_style::{Color, ComputedStyle, DisplayType, FontSize};

pub fn create_layout_object(
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    if let Some(n) = node {
        let layout_object = Rc::new(RefCell::new(LayoutObject::new(n.clone(), parent_obj)));

        for rule in &cssom.rules {
            if layout_object.borrow().is_node_selected(&rule.selector) {
                layout_object
                    .borrow_mut()
                    .cascading_style(rule.declarations.clone());
            }
        }

        let parent_style = if let Some(parent) = parent_obj {
            Some(parent.borrow().style())
        } else {
            None
        };
        layout_object.borrow_mut().defaulting_style(n, parent_style);

        if layout_object.borrow().style.display() == DisplayType::DisplayNone {
            return None;
        }

        layout_object.borrow_mut().update_kind();
        return Some(layout_object);
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct LayoutObject {
    kind: LayoutObjectKind,
    node: Rc<RefCell<Node>>,
    first_child: Option<Rc<RefCell<LayoutObject>>>,
    next_sibling: Option<Rc<RefCell<LayoutObject>>>,
    parent: Weak<RefCell<LayoutObject>>,
    style: ComputedStyle,
    point: LayoutPoint,
    size: LayoutSize,
}

impl LayoutObject {
    pub fn new(node: Rc<RefCell<Node>>, parent_obj: &Option<Rc<RefCell<LayoutObject>>>) -> Self {
        let parent = match parent_obj {
            Some(p) => Rc::downgrade(p),
            None => Weak::new(),
        };

        Self {
            kind: LayoutObjectKind::Block,
            node: node.clone(),
            first_child: None,
            next_sibling: None,
            parent,
            style: ComputedStyle::new(),
            point: LayoutPoint { x: 0, y: 0 },
            size: LayoutSize {
                width: 0,
                height: 0,
            },
        }
    }

    pub fn kind(&self) -> LayoutObjectKind {
        self.kind
    }

    pub fn node_kind(&self) -> NodeKind {
        self.node.borrow().kind().clone()
    }

    pub fn set_first_child(&mut self, first_child: Option<Rc<RefCell<LayoutObject>>>) {
        self.first_child = first_child;
    }

    pub fn first_child(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.first_child.clone()
    }

    pub fn set_next_sibling(&mut self, next_sibling: Option<Rc<RefCell<LayoutObject>>>) {
        self.next_sibling = next_sibling;
    }

    pub fn next_sibling(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.next_sibling.clone()
    }

    pub fn parent(&self) -> Weak<RefCell<Self>> {
        self.parent.clone()
    }

    pub fn style(&self) -> ComputedStyle {
        self.style.clone()
    }

    pub fn point(&self) -> LayoutPoint {
        self.point
    }

    pub fn size(&self) -> LayoutSize {
        self.size
    }

    pub fn is_node_selected(&self, selector: &Selector) -> bool {
        match &self.node_kind() {
            NodeKind::Element(e) => match selector {
                Selector::TypeSelector(type_name) => {
                    if e.kind().to_string() == *type_name {
                        return true;
                    }

                    false
                }
                Selector::ClassSelector(class_name) => {
                    for attr in &e.attributes() {
                        if attr.name() == "class" && attr.value() == *class_name {
                            return true;
                        }
                    }

                    false
                }
                Selector::IdSelector(id_name) => {
                    for attr in &e.attributes() {
                        if attr.name() == "id" && attr.value() == *id_name {
                            return true;
                        }
                    }

                    false
                }
                Selector::UnknownSelector => false,
            },
            _ => false,
        }
    }

    pub fn cascading_style(&mut self, declarations: Vec<Declaration>) {
        for declaration in declarations {
            match declaration.property.as_str() {
                "background-color" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let color = Color::from_name(&value).unwrap_or(Color::white());
                        self.style.set_background_color(color);

                        continue;
                    }

                    if let ComponentValue::HashToken(color_code) = &declaration.value {
                        let color = Color::from_code(&color_code).unwrap_or(Color::white());
                        self.style.set_background_color(color);

                        continue;
                    }
                }
                "color" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let color = Color::from_name(&value).unwrap_or(Color::black());
                        self.style.set_color(color);

                        continue;
                    }

                    if let ComponentValue::HashToken(color_code) = &declaration.value {
                        let color = Color::from_code(&color_code).unwrap_or(Color::black());
                        self.style.set_color(color);

                        continue;
                    }
                }
                "display" => {
                    if let ComponentValue::Ident(value) = declaration.value {
                        let display_type =
                            DisplayType::from_str(&value).unwrap_or(DisplayType::DisplayNone);
                        self.style.set_display(display_type);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn defaulting_style(
        &mut self,
        node: &Rc<RefCell<Node>>,
        parent_style: Option<ComputedStyle>,
    ) {
        self.style.defaulting(node, parent_style);
    }

    pub fn update_kind(&mut self) {
        self.kind = match self.node_kind() {
            NodeKind::Document => panic!("should not create a layout object for a Document node"),
            NodeKind::Element(_) => match self.style.display() {
                DisplayType::Block => LayoutObjectKind::Block,
                DisplayType::Inline => LayoutObjectKind::Inline,
                DisplayType::DisplayNone => {
                    panic!("should not create a layout object for display: none");
                }
            },
            NodeKind::Text(_) => LayoutObjectKind::Text,
        }
    }

    pub fn compute_size(&mut self, parent_size: LayoutSize) {
        self.size = match self.kind {
            LayoutObjectKind::Block => {
                let mut height = 0;
                let mut child = self.first_child();
                let mut previous_child_kind = LayoutObjectKind::Block;

                while let Some(c) = child {
                    if previous_child_kind == LayoutObjectKind::Block
                        || c.borrow().kind == LayoutObjectKind::Block
                    {
                        height += c.borrow().size.height;
                    }

                    previous_child_kind = c.borrow().kind();
                    child = c.borrow().next_sibling();
                }

                LayoutSize {
                    width: parent_size.width,
                    height,
                }
            }
            LayoutObjectKind::Inline => {
                let mut width = 0;
                let mut height = 0;
                let mut child = self.first_child();

                while let Some(c) = child {
                    width += c.borrow().size.width;
                    height += c.borrow().size.height;
                    child = c.borrow().next_sibling();
                }

                LayoutSize { width, height }
            }
            LayoutObjectKind::Text => {
                let NodeKind::Text(t) = self.node_kind() else {
                    panic!("node kind should be Text")
                };
                let ratio = match self.style.font_size() {
                    FontSize::Medium => 1,
                    FontSize::XLarge => 2,
                    FontSize::XXLarge => 3,
                };
                let width = CHAR_WIDTH * ratio * t.len() as i64;

                if width > CONTENT_AREA_WIDTH {
                    let line_num = if width % CONTENT_AREA_WIDTH == 0 {
                        width / CONTENT_AREA_WIDTH
                    } else {
                        width / CONTENT_AREA_WIDTH + 1
                    };

                    LayoutSize {
                        width: CONTENT_AREA_WIDTH,
                        height: CHAR_HEIGHT_WITH_PADDING * ratio * line_num,
                    }
                } else {
                    LayoutSize {
                        width,
                        height: CHAR_HEIGHT_WITH_PADDING * ratio,
                    }
                }
            }
        }
    }

    pub fn compute_position(
        &mut self,
        parent_point: LayoutPoint,
        previous_sibling_kind: LayoutObjectKind,
        previous_sibling_point: Option<LayoutPoint>,
        previous_sibling_size: Option<LayoutSize>,
    ) {
        self.point = if let (Some(sibling_size), Some(sibling_pos)) =
            (previous_sibling_size, previous_sibling_point)
        {
            match (self.kind, previous_sibling_kind) {
                (LayoutObjectKind::Block, _) | (_, LayoutObjectKind::Block) => LayoutPoint {
                    x: parent_point.x,
                    y: sibling_pos.y + sibling_size.height,
                },
                (LayoutObjectKind::Inline, LayoutObjectKind::Inline) => LayoutPoint {
                    x: sibling_pos.x + sibling_size.width,
                    y: sibling_pos.y,
                },
                _ => parent_point,
            }
        } else {
            parent_point
        }
    }

    pub fn paint(&mut self) -> Vec<DisplayItem> {
        if self.style.display() == DisplayType::DisplayNone {
            return vec![];
        }

        match self.kind {
            LayoutObjectKind::Block => {
                if let NodeKind::Element(_) = self.node_kind() {
                    return vec![DisplayItem::Rect {
                        style: self.style(),
                        layout_point: self.point(),
                        layout_size: self.size(),
                    }];
                }
            }
            LayoutObjectKind::Inline => {
                todo!("インライン要素を描画する");
            }
            LayoutObjectKind::Text => {
                if let NodeKind::Text(t) = self.node_kind() {
                    let mut v = vec![];

                    let ratio = match self.style.font_size() {
                        FontSize::Medium => 1,
                        FontSize::XLarge => 2,
                        FontSize::XXLarge => 3,
                    };
                    let plain_text = t
                        .replace("\n", " ")
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let lines = split_text(plain_text, CHAR_WIDTH * ratio);
                    let mut i = 0;
                    for line in lines {
                        let item = DisplayItem::Text {
                            text: line,
                            style: self.style(),
                            layout_point: LayoutPoint {
                                x: self.point.x,
                                y: self.point.y + CHAR_HEIGHT_WITH_PADDING * i,
                            },
                        };
                        v.push(item);
                        i += 1;
                    }

                    return v;
                }
            }
        }

        vec![]
    }
}

fn split_text(line: String, char_width: i64) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    if line.len() as i64 * char_width > (WINDOW_WIDTH + WINDOW_PADDING) {
        let s = line.split_at(find_index_for_line_break(
            &line,
            ((WINDOW_WIDTH + WINDOW_PADDING) / char_width) as usize,
        ));
        result.push(s.0.to_string());
        result.extend(split_text(s.1.trim().to_string(), char_width));
    } else {
        result.push(line);
    }

    result
}

fn find_index_for_line_break(line: &String, max_index: usize) -> usize {
    let line: Vec<char> = line.chars().collect();

    for i in (0..max_index).rev() {
        if line[i] == ' ' {
            return i;
        }
    }

    max_index
}

impl PartialEq for LayoutObject {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LayoutObjectKind {
    Block,
    Inline,
    Text,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct LayoutPoint {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct LayoutSize {
    pub width: i64,
    pub height: i64,
}

