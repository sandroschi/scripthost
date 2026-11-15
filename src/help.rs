use std::io::BufRead;

#[cfg(windows)]
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &'static str = "\n";
const TAB: &'static str = "  ";

pub struct Help {
    nodes: Vec<Node>,
}

pub struct Node {
    name: String,
    methods: Vec<Method>,
}

pub struct Method {
    signature: String,
    examples: Vec<Example>,
}

pub struct Example {
    call: String,
    result: Option<String>,
}

impl Help {
    pub fn new() -> Help {
        Help { nodes: vec![] }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn to_string(&self) -> String {
        let mut lines:Vec<String> = vec![];
        
        for node in self.nodes.iter() {
            lines.append(&mut node.to_string());
        }

        lines.join(LINE_ENDING)
    }
}

impl Node {
    pub fn new(name: String) -> Node {
        Node { name, methods: vec![] }
    }

    pub fn add_method(&mut self, method: Method) {
        self.methods.push(method);
    }
    
    fn to_string(&self) -> Vec<String> {
        let mut lines = vec![];
        for method in self.methods.iter() {
            let method_lines  = method.to_string();
            for line in method_lines {
                lines.push(format!("{}{}", TAB, line));
            }
        }
        lines
    }
}

impl Method {
    pub fn new(signature: String) -> Method {
        Method { signature, examples: vec![] }
    }

    pub fn add_example(&mut self, example: Example) {
        self.examples.push(example);
    }
    
    fn to_string(&self) -> Vec<String> {
        let mut lines = vec![];
        lines.push(self.signature.clone());
        for example in self.examples.iter() {
            lines.push(format!("{}{}", TAB, &mut example.to_string()));
        }
        lines
    }
}

impl Example {
    pub fn new(call: String, result: Option<String>) -> Example {
        Example { call, result }
    }
    
    fn to_string(&self) -> String {
        if let Some(result) = &self.result {
            format!("{} -> {}", self.call.clone(), result.clone())    
        } else { 
            self.call.clone()
        }
    }
}

