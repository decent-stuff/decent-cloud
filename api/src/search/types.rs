/// AST types for the search DSL

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Eq,    // field:value
    Gte,   // field:>=value
    Lte,   // field:<=value
    Gt,    // field:>value
    Lt,    // field:<value
    Range, // field:[min TO max]
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub field: String,
    pub operator: Operator,
    pub values: Vec<Value>, // Multiple values for OR groups
    pub negated: bool,
}

impl Filter {
    pub fn new(field: String, operator: Operator, values: Vec<Value>, negated: bool) -> Self {
        Self {
            field,
            operator,
            values,
            negated,
        }
    }
}
