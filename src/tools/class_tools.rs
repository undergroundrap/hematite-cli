use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "name": "class_tools",
        "description": "Inspect Java .class bytecode files without javap or a JDK. 5 actions: info (default — class name/type/access/superclass/interfaces/Java version/constant pool breakdown/field+method counts), methods (all methods with decoded signature and access flags), fields (all fields with decoded type and access flags), constants (class references + string literals from constant pool), imports (all referenced classes categorized as Java stdlib vs other). Pass 'file' (path to .class file) or 'hex' (hex-encoded class bytes).",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "methods", "fields", "constants", "imports"],
                    "description": "Operation to perform (default: info)"
                },
                "file": { "type": "string", "description": "Path to a .class file" },
                "hex": { "type": "string", "description": "Hex-encoded .class bytes" }
            }
        }
    })
}

const CLASS_MAGIC: u32 = 0xCAFE_BABE;

const CP_UTF8: u8 = 1;
const CP_INTEGER: u8 = 3;
const CP_FLOAT: u8 = 4;
const CP_LONG: u8 = 5;
const CP_DOUBLE: u8 = 6;
const CP_CLASS: u8 = 7;
const CP_STRING: u8 = 8;
const CP_FIELDREF: u8 = 9;
const CP_METHODREF: u8 = 10;
const CP_INTERFACE_METHODREF: u8 = 11;
const CP_NAME_AND_TYPE: u8 = 12;
const CP_METHOD_HANDLE: u8 = 15;
const CP_METHOD_TYPE: u8 = 16;
const CP_DYNAMIC: u8 = 17;
const CP_INVOKE_DYNAMIC: u8 = 18;
const CP_MODULE: u8 = 19;
const CP_PACKAGE: u8 = 20;

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("unexpected end of file".to_string());
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u16_be(&mut self) -> Result<u16, String> {
        if self.pos + 2 > self.data.len() {
            return Err("unexpected end of file reading u16".to_string());
        }
        let v = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn read_u32_be(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.data.len() {
            return Err("unexpected end of file reading u32".to_string());
        }
        let v = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, String> {
        if self.pos + n > self.data.len() {
            return Err(format!("unexpected end of file reading {} bytes", n));
        }
        let v = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum CpEntry {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    StringRef(u16),
    Fieldref(u16, u16),
    Methodref(u16, u16),
    InterfaceMethodref(u16, u16),
    NameAndType(u16, u16),
    MethodHandle(u8, u16),
    MethodType(u16),
    Dynamic(u16, u16),
    InvokeDynamic(u16, u16),
    Module(u16),
    Package(u16),
    Placeholder,
}

#[allow(dead_code)]
struct MemberInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attribute_count: u16,
}

struct AttrInfo {
    name_index: u16,
    data: Vec<u8>,
}

struct ClassFile {
    minor: u16,
    major: u16,
    pool: Vec<CpEntry>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<MemberInfo>,
    methods: Vec<MemberInfo>,
    attribute_count: u16,
    attributes: Vec<AttrInfo>,
}

fn parse_member(r: &mut Reader) -> Result<MemberInfo, String> {
    let access_flags = r.read_u16_be()?;
    let name_index = r.read_u16_be()?;
    let descriptor_index = r.read_u16_be()?;
    let attr_count = r.read_u16_be()?;
    for _ in 0..attr_count {
        let _name_index = r.read_u16_be()?;
        let len = r.read_u32_be()? as usize;
        r.read_bytes(len)?;
    }
    Ok(MemberInfo {
        access_flags,
        name_index,
        descriptor_index,
        attribute_count: attr_count,
    })
}

fn parse_class(data: &[u8]) -> Result<ClassFile, String> {
    let mut r = Reader::new(data);

    let magic = r.read_u32_be()?;
    if magic != CLASS_MAGIC {
        return Err(format!(
            "not a Java .class file (magic: 0x{:08X}, expected 0xCAFEBABE)",
            magic
        ));
    }

    let minor = r.read_u16_be()?;
    let major = r.read_u16_be()?;

    let cp_count = r.read_u16_be()?;
    let mut pool: Vec<CpEntry> = Vec::with_capacity(cp_count as usize);
    pool.push(CpEntry::Placeholder); // index 0 is reserved

    let mut i = 1u16;
    while i < cp_count {
        let tag = r.read_u8()?;
        let entry = match tag {
            CP_UTF8 => {
                let len = r.read_u16_be()? as usize;
                let bytes = r.read_bytes(len)?;
                CpEntry::Utf8(String::from_utf8_lossy(&bytes).into_owned())
            }
            CP_INTEGER => {
                let b = r.read_bytes(4)?;
                CpEntry::Integer(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            }
            CP_FLOAT => {
                let b = r.read_bytes(4)?;
                CpEntry::Float(f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            }
            CP_LONG => {
                let b = r.read_bytes(8)?;
                pool.push(CpEntry::Long(i64::from_be_bytes([
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                ])));
                pool.push(CpEntry::Placeholder);
                i += 2;
                continue;
            }
            CP_DOUBLE => {
                let b = r.read_bytes(8)?;
                pool.push(CpEntry::Double(f64::from_be_bytes([
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                ])));
                pool.push(CpEntry::Placeholder);
                i += 2;
                continue;
            }
            CP_CLASS => CpEntry::Class(r.read_u16_be()?),
            CP_STRING => CpEntry::StringRef(r.read_u16_be()?),
            CP_FIELDREF => CpEntry::Fieldref(r.read_u16_be()?, r.read_u16_be()?),
            CP_METHODREF => CpEntry::Methodref(r.read_u16_be()?, r.read_u16_be()?),
            CP_INTERFACE_METHODREF => {
                CpEntry::InterfaceMethodref(r.read_u16_be()?, r.read_u16_be()?)
            }
            CP_NAME_AND_TYPE => CpEntry::NameAndType(r.read_u16_be()?, r.read_u16_be()?),
            CP_METHOD_HANDLE => CpEntry::MethodHandle(r.read_u8()?, r.read_u16_be()?),
            CP_METHOD_TYPE => CpEntry::MethodType(r.read_u16_be()?),
            CP_DYNAMIC => CpEntry::Dynamic(r.read_u16_be()?, r.read_u16_be()?),
            CP_INVOKE_DYNAMIC => CpEntry::InvokeDynamic(r.read_u16_be()?, r.read_u16_be()?),
            CP_MODULE => CpEntry::Module(r.read_u16_be()?),
            CP_PACKAGE => CpEntry::Package(r.read_u16_be()?),
            _ => return Err(format!("unknown constant pool tag: {}", tag)),
        };
        pool.push(entry);
        i += 1;
    }

    let access_flags = r.read_u16_be()?;
    let this_class = r.read_u16_be()?;
    let super_class = r.read_u16_be()?;

    let iface_count = r.read_u16_be()?;
    let mut interfaces = Vec::with_capacity(iface_count as usize);
    for _ in 0..iface_count {
        interfaces.push(r.read_u16_be()?);
    }

    let fields_count = r.read_u16_be()?;
    let mut fields = Vec::with_capacity(fields_count as usize);
    for _ in 0..fields_count {
        fields.push(parse_member(&mut r)?);
    }

    let methods_count = r.read_u16_be()?;
    let mut methods = Vec::with_capacity(methods_count as usize);
    for _ in 0..methods_count {
        methods.push(parse_member(&mut r)?);
    }

    let attribute_count = r.read_u16_be()?;
    let mut attributes = Vec::with_capacity(attribute_count as usize);
    for _ in 0..attribute_count {
        let name_index = r.read_u16_be()?;
        let len = r.read_u32_be()? as usize;
        let data = r.read_bytes(len)?;
        attributes.push(AttrInfo { name_index, data });
    }

    Ok(ClassFile {
        minor,
        major,
        pool,
        access_flags,
        this_class,
        super_class,
        interfaces,
        fields,
        methods,
        attribute_count,
        attributes,
    })
}

fn get_utf8(pool: &[CpEntry], idx: u16) -> String {
    if idx == 0 || idx as usize >= pool.len() {
        return format!("<cp#{}>", idx);
    }
    match &pool[idx as usize] {
        CpEntry::Utf8(s) => s.clone(),
        _ => format!("<not-utf8@cp#{}>", idx),
    }
}

fn get_class_name(pool: &[CpEntry], idx: u16) -> String {
    if idx == 0 {
        return "(none)".to_string();
    }
    if idx as usize >= pool.len() {
        return format!("<cp#{}>", idx);
    }
    match &pool[idx as usize] {
        CpEntry::Class(name_idx) => get_utf8(pool, *name_idx).replace('/', "."),
        _ => format!("<not-class@cp#{}>", idx),
    }
}

fn java_version_label(major: u16) -> &'static str {
    match major {
        45 => "Java 1.1",
        46 => "Java 1.2",
        47 => "Java 1.3",
        48 => "Java 1.4",
        49 => "Java 5",
        50 => "Java 6",
        51 => "Java 7",
        52 => "Java 8",
        53 => "Java 9",
        54 => "Java 10",
        55 => "Java 11",
        56 => "Java 12",
        57 => "Java 13",
        58 => "Java 14",
        59 => "Java 15",
        60 => "Java 16",
        61 => "Java 17",
        62 => "Java 18",
        63 => "Java 19",
        64 => "Java 20",
        65 => "Java 21",
        66 => "Java 22",
        67 => "Java 23",
        68 => "Java 24",
        m if m > 68 => "Java 25+",
        _ => "Java pre-1.1",
    }
}

fn class_access_label(flags: u16) -> String {
    let mut parts = Vec::new();
    if flags & 0x0001 != 0 {
        parts.push("public");
    }
    if flags & 0x0010 != 0 {
        parts.push("final");
    }
    if flags & 0x0200 != 0 {
        parts.push("interface");
    }
    if flags & 0x0400 != 0 {
        parts.push("abstract");
    }
    if flags & 0x1000 != 0 {
        parts.push("synthetic");
    }
    if flags & 0x2000 != 0 {
        parts.push("annotation");
    }
    if flags & 0x4000 != 0 {
        parts.push("enum");
    }
    if flags & 0x8000 != 0 {
        parts.push("module");
    }
    if parts.is_empty() {
        "package-private".to_string()
    } else {
        parts.join(" ")
    }
}

fn member_access_label(flags: u16, is_method: bool) -> String {
    let mut parts = Vec::new();
    if flags & 0x0001 != 0 {
        parts.push("public");
    }
    if flags & 0x0002 != 0 {
        parts.push("private");
    }
    if flags & 0x0004 != 0 {
        parts.push("protected");
    }
    if flags & 0x0008 != 0 {
        parts.push("static");
    }
    if flags & 0x0010 != 0 {
        parts.push("final");
    }
    if is_method {
        if flags & 0x0020 != 0 {
            parts.push("synchronized");
        }
        if flags & 0x0040 != 0 {
            parts.push("bridge");
        }
        if flags & 0x0080 != 0 {
            parts.push("varargs");
        }
        if flags & 0x0100 != 0 {
            parts.push("native");
        }
        if flags & 0x0400 != 0 {
            parts.push("abstract");
        }
        if flags & 0x0800 != 0 {
            parts.push("strictfp");
        }
    } else {
        if flags & 0x0040 != 0 {
            parts.push("volatile");
        }
        if flags & 0x0080 != 0 {
            parts.push("transient");
        }
        if flags & 0x4000 != 0 {
            parts.push("enum");
        }
    }
    if flags & 0x1000 != 0 {
        parts.push("synthetic");
    }
    if parts.is_empty() {
        "package-private".to_string()
    } else {
        parts.join(" ")
    }
}

fn decode_descriptor(desc: &str) -> String {
    if desc.starts_with('(') {
        if let Some(close) = desc.find(')') {
            let params_raw = &desc[1..close];
            let ret_raw = &desc[close + 1..];
            let params = decode_type_list(params_raw);
            let ret = decode_single_type(ret_raw);
            if params.is_empty() {
                format!("() -> {}", ret)
            } else {
                format!("({}) -> {}", params.join(", "), ret)
            }
        } else {
            desc.to_string()
        }
    } else {
        decode_single_type(desc)
    }
}

fn decode_single_type(s: &str) -> String {
    match s.chars().next() {
        Some('B') => "byte".to_string(),
        Some('C') => "char".to_string(),
        Some('D') => "double".to_string(),
        Some('F') => "float".to_string(),
        Some('I') => "int".to_string(),
        Some('J') => "long".to_string(),
        Some('S') => "short".to_string(),
        Some('Z') => "boolean".to_string(),
        Some('V') => "void".to_string(),
        Some('[') => format!("{}[]", decode_single_type(&s[1..])),
        Some('L') => {
            let inner = s[1..].trim_end_matches(';');
            inner.replace('/', ".")
        }
        _ => s.to_string(),
    }
}

fn decode_type_list(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b'V' => {
                result.push(decode_single_type(&s[i..i + 1]));
                i += 1;
            }
            b'[' => {
                let mut j = i;
                while j < bytes.len() && bytes[j] == b'[' {
                    j += 1;
                }
                let dims = j - i;
                let (base, base_end) = if j < bytes.len() && bytes[j] == b'L' {
                    let end = s[j..].find(';').map(|k| j + k + 1).unwrap_or(bytes.len());
                    (decode_single_type(&s[j..end]), end)
                } else if j < bytes.len() {
                    (decode_single_type(&s[j..j + 1]), j + 1)
                } else {
                    break;
                };
                result.push(format!("{}{}", base, "[]".repeat(dims)));
                i = base_end;
            }
            b'L' => {
                if let Some(end) = s[i..].find(';') {
                    let class_name = s[i + 1..i + end].replace('/', ".");
                    result.push(class_name);
                    i += end + 1;
                } else {
                    break;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    result
}

fn cp_type_label(entry: &CpEntry) -> &'static str {
    match entry {
        CpEntry::Utf8(_) => "Utf8",
        CpEntry::Integer(_) => "Integer",
        CpEntry::Float(_) => "Float",
        CpEntry::Long(_) => "Long",
        CpEntry::Double(_) => "Double",
        CpEntry::Class(_) => "Class",
        CpEntry::StringRef(_) => "String",
        CpEntry::Fieldref(_, _) => "Fieldref",
        CpEntry::Methodref(_, _) => "Methodref",
        CpEntry::InterfaceMethodref(_, _) => "InterfaceMethodref",
        CpEntry::NameAndType(_, _) => "NameAndType",
        CpEntry::MethodHandle(_, _) => "MethodHandle",
        CpEntry::MethodType(_) => "MethodType",
        CpEntry::Dynamic(_, _) => "Dynamic",
        CpEntry::InvokeDynamic(_, _) => "InvokeDynamic",
        CpEntry::Module(_) => "Module",
        CpEntry::Package(_) => "Package",
        CpEntry::Placeholder => "Placeholder",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn action_info(cf: &ClassFile) -> String {
    let mut out = String::new();
    out.push_str("=== Java .class Info ===\n\n");

    let class_name = get_class_name(&cf.pool, cf.this_class);
    let super_name = get_class_name(&cf.pool, cf.super_class);
    let version_str = java_version_label(cf.major);
    let flags_str = class_access_label(cf.access_flags);

    let type_label = if cf.access_flags & 0x2000 != 0 {
        "@interface"
    } else if cf.access_flags & 0x0200 != 0 {
        "interface"
    } else if cf.access_flags & 0x4000 != 0 {
        "enum"
    } else {
        "class"
    };

    out.push_str(&format!("Class:        {}\n", class_name));
    out.push_str(&format!("Type:         {}\n", type_label));
    out.push_str(&format!("Access:       {}\n", flags_str));
    let super_display = if super_name == "(none)" {
        "java.lang.Object".to_string()
    } else {
        super_name
    };
    out.push_str(&format!("Superclass:   {}\n", super_display));
    out.push_str(&format!(
        "Version:      {}.{} ({} — class file format {})\n",
        cf.major, cf.minor, version_str, cf.major
    ));

    if !cf.interfaces.is_empty() {
        out.push_str("\nImplements:\n");
        for &idx in &cf.interfaces {
            out.push_str(&format!("  {}\n", get_class_name(&cf.pool, idx)));
        }
    }

    out.push_str(&format!(
        "\nConstant pool:  {} entries\n",
        cf.pool.len() - 1
    ));
    out.push_str(&format!("Fields:         {}\n", cf.fields.len()));
    out.push_str(&format!("Methods:        {}\n", cf.methods.len()));
    out.push_str(&format!("Attributes:     {}\n", cf.attribute_count));

    // Source file attribute if present
    for attr in &cf.attributes {
        let attr_name = get_utf8(&cf.pool, attr.name_index);
        if attr_name == "SourceFile" && attr.data.len() >= 2 {
            let src_idx = u16::from_be_bytes([attr.data[0], attr.data[1]]);
            let src = get_utf8(&cf.pool, src_idx);
            out.push_str(&format!("Source file:    {}\n", src));
        }
    }

    // Constant pool summary
    let mut cp_counts: HashMap<&str, u32> = HashMap::new();
    for entry in &cf.pool {
        if matches!(entry, CpEntry::Placeholder) {
            continue;
        }
        *cp_counts.entry(cp_type_label(entry)).or_insert(0) += 1;
    }

    out.push_str("\nConstant Pool Breakdown:\n");
    let mut cp_vec: Vec<_> = cp_counts.iter().collect();
    cp_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in &cp_vec {
        out.push_str(&format!("  {:25} {}\n", label, count));
    }

    out
}

fn action_methods(cf: &ClassFile) -> String {
    let mut out = String::new();
    out.push_str("=== Methods ===\n\n");

    if cf.methods.is_empty() {
        out.push_str("(no methods)\n");
        return out;
    }

    out.push_str(&format!(
        "{:<45} {:<35} {}\n",
        "Method", "Signature", "Access"
    ));
    out.push_str(&format!("{}\n", "-".repeat(110)));

    for m in &cf.methods {
        let name = get_utf8(&cf.pool, m.name_index);
        let desc = get_utf8(&cf.pool, m.descriptor_index);
        let decoded = decode_descriptor(&desc);
        let flags = member_access_label(m.access_flags, true);
        out.push_str(&format!(
            "{:<45} {:<35} {}\n",
            truncate(&name, 42),
            truncate(&decoded, 32),
            flags
        ));
    }

    out
}

fn action_fields(cf: &ClassFile) -> String {
    let mut out = String::new();
    out.push_str("=== Fields ===\n\n");

    if cf.fields.is_empty() {
        out.push_str("(no fields)\n");
        return out;
    }

    out.push_str(&format!("{:<45} {:<22} {}\n", "Field", "Type", "Access"));
    out.push_str(&format!("{}\n", "-".repeat(90)));

    for f in &cf.fields {
        let name = get_utf8(&cf.pool, f.name_index);
        let desc = get_utf8(&cf.pool, f.descriptor_index);
        let type_str = decode_single_type(&desc);
        let flags = member_access_label(f.access_flags, false);
        out.push_str(&format!(
            "{:<45} {:<22} {}\n",
            truncate(&name, 42),
            truncate(&type_str, 19),
            flags
        ));
    }

    out
}

fn action_constants(cf: &ClassFile) -> String {
    let mut out = String::new();
    out.push_str("=== Constant Pool ===\n\n");

    // Class references
    let mut classes: Vec<String> = Vec::new();
    for entry in &cf.pool {
        if let CpEntry::Class(name_idx) = entry {
            let name = get_utf8(&cf.pool, *name_idx);
            if !name.starts_with('[') {
                classes.push(name.replace('/', "."));
            }
        }
    }
    classes.sort();
    classes.dedup();

    out.push_str(&format!("Class references ({}):\n", classes.len()));
    for c in &classes {
        out.push_str(&format!("  {}\n", c));
    }

    // String literals (first 30)
    let mut strings: Vec<String> = Vec::new();
    for entry in &cf.pool {
        if let CpEntry::StringRef(str_idx) = entry {
            let s = get_utf8(&cf.pool, *str_idx);
            if !s.is_empty() {
                strings.push(s);
            }
        }
    }

    if !strings.is_empty() {
        out.push_str(&format!(
            "\nString literals ({} total, first 30):\n",
            strings.len()
        ));
        for s in strings.iter().take(30) {
            let display = s
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            out.push_str(&format!("  \"{}\"\n", truncate(&display, 100)));
        }
    }

    out
}

fn action_imports(cf: &ClassFile) -> String {
    let mut out = String::new();
    out.push_str("=== Referenced Classes ===\n\n");

    let this_name = get_class_name(&cf.pool, cf.this_class);

    let mut classes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in &cf.pool {
        if let CpEntry::Class(name_idx) = entry {
            let name = get_utf8(&cf.pool, *name_idx);
            let resolved = if name.starts_with('[') {
                // Array — extract element class if object type
                let elem = name.trim_start_matches('[');
                if let Some(stripped) = elem.strip_prefix('L') {
                    stripped.trim_end_matches(';').replace('/', ".")
                } else {
                    continue;
                }
            } else {
                name.replace('/', ".")
            };
            if resolved != this_name {
                classes.insert(resolved);
            }
        }
    }

    let mut java_stdlib: Vec<String> = Vec::new();
    let mut third_party: Vec<String> = Vec::new();

    for c in &classes {
        if c.starts_with("java.")
            || c.starts_with("javax.")
            || c.starts_with("sun.")
            || c.starts_with("com.sun.")
        {
            java_stdlib.push(c.clone());
        } else {
            third_party.push(c.clone());
        }
    }

    java_stdlib.sort();
    third_party.sort();

    if !java_stdlib.is_empty() {
        out.push_str(&format!("Java Standard Library ({}):\n", java_stdlib.len()));
        for c in &java_stdlib {
            out.push_str(&format!("  {}\n", c));
        }
    }

    if !third_party.is_empty() {
        out.push_str(&format!("\nOther References ({}):\n", third_party.len()));
        for c in &third_party {
            out.push_str(&format!("  {}\n", c));
        }
    }

    if classes.is_empty() {
        out.push_str("(no class references found)\n");
    }

    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let bytes = if let Some(file_path) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read(file_path).map_err(|e| format!("cannot read file '{}': {}", file_path, e))?
    } else if let Some(hex_str) = args.get("hex").and_then(|v| v.as_str()) {
        let clean: String = hex_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if !clean.len().is_multiple_of(2) {
            return Err("hex string has odd length".to_string());
        }
        (0..clean.len() / 2)
            .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return Err(
            "provide 'file' (path to .class file) or 'hex' (hex-encoded class bytes)".to_string(),
        );
    };

    let cf = parse_class(&bytes)?;

    match action {
        "info" => Ok(action_info(&cf)),
        "methods" => Ok(action_methods(&cf)),
        "fields" => Ok(action_fields(&cf)),
        "constants" | "pool" => Ok(action_constants(&cf)),
        "imports" | "classes" => Ok(action_imports(&cf)),
        _ => Err(format!(
            "unknown action '{}'; use: info, methods, fields, constants, imports",
            action
        )),
    }
}
