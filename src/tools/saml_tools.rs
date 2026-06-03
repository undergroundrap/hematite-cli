use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "attributes", "validate", "explain"],
                "description": "Action: parse (default), attributes, validate, explain"
            },
            "xml":     { "type": "string", "description": "Raw SAML XML assertion or response" },
            "base64":  { "type": "string", "description": "Base64-encoded SAML XML (SAMLResponse from redirect/POST binding)" },
            "file":    { "type": "string", "description": "Path to SAML XML file" },
            "topic":   { "type": "string", "description": "Topic for explain action: bindings, assertions, idp, sp, sso, security" }
        }
    })
}

// ── XML helper ────────────────────────────────────────────────────────────────

struct XmlNode {
    tag: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<XmlNode>,
}

fn attr_val<'a>(node: &'a XmlNode, name: &str) -> Option<&'a str> {
    node.attrs.iter().find(|(k, _)| k == name || k.ends_with(&format!(":{}", name)) || k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn find_all<'a>(node: &'a XmlNode, local_name: &str) -> Vec<&'a XmlNode> {
    let mut out: Vec<&'a XmlNode> = Vec::new();
    if tag_local(&node.tag) == local_name {
        out.push(node);
    }
    for c in &node.children {
        out.extend(find_all(c, local_name));
    }
    out
}

fn find_first<'a>(node: &'a XmlNode, local_name: &str) -> Option<&'a XmlNode> {
    if tag_local(&node.tag) == local_name { return Some(node); }
    for c in &node.children {
        if let Some(found) = find_first(c, local_name) { return Some(found); }
    }
    None
}

fn tag_local(tag: &str) -> &str {
    if let Some(pos) = tag.rfind(':') { &tag[pos + 1..] } else { tag }
}

fn parse_xml_simple(xml: &str) -> Result<XmlNode, String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlNode> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = Vec::new();
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = attr.decode_and_unescape_value(reader.decoder()).unwrap_or_default().to_string();
                    attrs.push((key, val));
                }
                stack.push(XmlNode { tag, attrs, text: String::new(), children: Vec::new() });
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = Vec::new();
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = attr.decode_and_unescape_value(reader.decoder()).unwrap_or_default().to_string();
                    attrs.push((key, val));
                }
                let node = XmlNode { tag, attrs, text: String::new(), children: Vec::new() };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let node = stack.pop().unwrap();
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                } else {
                    break;
                }
            }
            Ok(Event::Text(t)) => {
                let text = t.unescape().unwrap_or_default().to_string();
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            Ok(Event::CData(c)) => {
                let text = String::from_utf8_lossy(c.as_ref()).to_string();
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    stack.pop().ok_or_else(|| "Empty XML document".to_string())
}

// ── Base64 decode ─────────────────────────────────────────────────────────────

fn base64_decode_standard(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let s = s.trim_end_matches('=');
    let decode_char = |c: u8| -> Option<u8> {
        TABLE.iter().position(|&x| x == c).map(|p| p as u8)
    };
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let a = decode_char(bytes[i])?;
        let b = decode_char(bytes[i + 1])?;
        out.push((a << 2) | (b >> 4));
        if i + 2 < bytes.len() {
            let c = decode_char(bytes[i + 2])?;
            out.push((b << 4) | (c >> 2));
            if i + 3 < bytes.len() {
                let d = decode_char(bytes[i + 3])?;
                out.push((c << 2) | d);
                i += 4;
            } else { i += 3; }
        } else { i += 2; }
    }
    Some(out)
}

fn load_saml(args: &Value) -> Result<String, String> {
    if let Some(xml) = args.get("xml").and_then(|v| v.as_str()) {
        return Ok(xml.to_string());
    }
    if let Some(b64) = args.get("base64").and_then(|v| v.as_str()) {
        let decoded = base64_decode_standard(b64)
            .ok_or("Cannot decode base64 input")?;
        return String::from_utf8(decoded).map_err(|_| "Decoded bytes are not valid UTF-8".to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read '{}': {}", path, e))?;
        // If the file content looks like base64, decode it
        let trimmed = raw.trim();
        if trimmed.chars().all(|c| c.is_alphanumeric() || "+/=\n\r ".contains(c)) && !trimmed.starts_with('<') {
            let decoded = base64_decode_standard(trimmed).ok_or("Cannot decode file as base64")?;
            return String::from_utf8(decoded).map_err(|_| "Decoded file is not valid UTF-8".to_string());
        }
        return Ok(raw);
    }
    Err("Provide 'xml' (raw SAML XML), 'base64' (SAMLResponse), or 'file' (path to XML/base64 file)".to_string())
}

// ── Parse ─────────────────────────────────────────────────────────────────────

struct SamlInfo {
    doc_type: String,
    version: String,
    id: String,
    in_response_to: String,
    issue_instant: String,
    issuer: String,
    destination: String,
    status_code: String,
    status_message: String,
    // Assertion fields
    assertion_id: String,
    subject_nameid: String,
    nameid_format: String,
    not_before: String,
    not_on_or_after: String,
    session_index: String,
    session_not_on_or_after: String,
    attributes: Vec<(String, Vec<String>)>,
    authn_context: String,
    signature_present: bool,
    encrypted_assertion: bool,
}

impl SamlInfo {
    fn new() -> Self {
        SamlInfo {
            doc_type: String::new(), version: String::new(), id: String::new(),
            in_response_to: String::new(), issue_instant: String::new(),
            issuer: String::new(), destination: String::new(),
            status_code: String::new(), status_message: String::new(),
            assertion_id: String::new(), subject_nameid: String::new(),
            nameid_format: String::new(), not_before: String::new(),
            not_on_or_after: String::new(), session_index: String::new(),
            session_not_on_or_after: String::new(), attributes: Vec::new(),
            authn_context: String::new(), signature_present: false, encrypted_assertion: false,
        }
    }
}

fn parse_saml_doc(root: &XmlNode) -> SamlInfo {
    let mut info = SamlInfo::new();
    let local = tag_local(&root.tag);
    info.doc_type = local.to_string();
    info.version = attr_val(root, "Version").unwrap_or("2.0").to_string();
    info.id = attr_val(root, "ID").unwrap_or("").to_string();
    info.in_response_to = attr_val(root, "InResponseTo").unwrap_or("").to_string();
    info.issue_instant = attr_val(root, "IssueInstant").unwrap_or("").to_string();
    info.destination = attr_val(root, "Destination").unwrap_or("").to_string();

    if let Some(issuer) = find_first(root, "Issuer") {
        info.issuer = issuer.text.clone();
    }

    if let Some(status) = find_first(root, "Status") {
        if let Some(sc) = find_first(status, "StatusCode") {
            let val = attr_val(sc, "Value").unwrap_or("");
            // Strip urn:oasis prefix
            info.status_code = val.rsplit(':').next().unwrap_or(val).to_string();
        }
        if let Some(sm) = find_first(status, "StatusMessage") {
            info.status_message = sm.text.clone();
        }
    }

    info.signature_present = find_first(root, "Signature").is_some();
    info.encrypted_assertion = find_first(root, "EncryptedAssertion").is_some();

    if let Some(assertion) = find_first(root, "Assertion") {
        info.assertion_id = attr_val(assertion, "ID").unwrap_or("").to_string();

        // Subject
        if let Some(subject) = find_first(assertion, "Subject") {
            if let Some(nameid) = find_first(subject, "NameID") {
                info.subject_nameid = nameid.text.clone();
                info.nameid_format = attr_val(nameid, "Format").unwrap_or("").rsplit(':').next().unwrap_or("").to_string();
            }
        }

        // Conditions
        if let Some(cond) = find_first(assertion, "Conditions") {
            info.not_before = attr_val(cond, "NotBefore").unwrap_or("").to_string();
            info.not_on_or_after = attr_val(cond, "NotOnOrAfter").unwrap_or("").to_string();
        }

        // AuthnStatement
        if let Some(authn) = find_first(assertion, "AuthnStatement") {
            info.session_index = attr_val(authn, "SessionIndex").unwrap_or("").to_string();
            info.session_not_on_or_after = attr_val(authn, "SessionNotOnOrAfter").unwrap_or("").to_string();
            if let Some(ctx) = find_first(authn, "AuthnContextClassRef") {
                info.authn_context = ctx.text.rsplit(':').next().unwrap_or(&ctx.text).to_string();
            }
        }

        // Attributes
        for attr_stmt in find_all(assertion, "AttributeStatement") {
            for attr in find_all(attr_stmt, "Attribute") {
                let name = attr_val(attr, "Name").unwrap_or("").to_string();
                let values: Vec<String> = find_all(attr, "AttributeValue")
                    .into_iter().map(|v| v.text.clone()).collect();
                info.attributes.push((name, values));
            }
        }
    }

    info
}

fn action_parse(args: &Value) -> Result<String, String> {
    let xml = load_saml(args)?;
    let root = parse_xml_simple(&xml)?;
    let info = parse_saml_doc(&root);

    let mut out = format!("## SAML {} Document\n\n", info.doc_type);
    out.push_str(&format!("  Version:         {}\n", info.version));
    out.push_str(&format!("  Document ID:     {}\n", info.id));
    if !info.in_response_to.is_empty() {
        out.push_str(&format!("  InResponseTo:    {}\n", info.in_response_to));
    }
    out.push_str(&format!("  IssueInstant:    {}\n", info.issue_instant));
    out.push_str(&format!("  Issuer:          {}\n", info.issuer));
    if !info.destination.is_empty() {
        out.push_str(&format!("  Destination:     {}\n", info.destination));
    }

    if !info.status_code.is_empty() {
        let icon = if info.status_code == "Success" { "✓" } else { "✗" };
        out.push_str(&format!("  Status:          {} {}\n", icon, info.status_code));
        if !info.status_message.is_empty() {
            out.push_str(&format!("  Status message:  {}\n", info.status_message));
        }
    }

    out.push_str(&format!("  Signature:       {}\n", if info.signature_present { "✓ Present" } else { "✗ Missing" }));
    if info.encrypted_assertion {
        out.push_str("  Assertion:       Encrypted (cannot read attributes without SP private key)\n");
        return Ok(out);
    }

    if !info.subject_nameid.is_empty() {
        out.push_str("\n## Subject\n\n");
        out.push_str(&format!("  NameID:          {}\n", info.subject_nameid));
        if !info.nameid_format.is_empty() {
            out.push_str(&format!("  Format:          {}\n", info.nameid_format));
        }
    }

    if !info.not_before.is_empty() || !info.not_on_or_after.is_empty() {
        out.push_str("\n## Validity Window\n\n");
        if !info.not_before.is_empty() {
            out.push_str(&format!("  NotBefore:       {}\n", info.not_before));
        }
        if !info.not_on_or_after.is_empty() {
            out.push_str(&format!("  NotOnOrAfter:    {}\n", info.not_on_or_after));
        }
    }

    if !info.session_index.is_empty() {
        out.push_str("\n## Session\n\n");
        out.push_str(&format!("  SessionIndex:    {}\n", info.session_index));
        if !info.session_not_on_or_after.is_empty() {
            out.push_str(&format!("  SessionExpires:  {}\n", info.session_not_on_or_after));
        }
        if !info.authn_context.is_empty() {
            out.push_str(&format!("  AuthnContext:    {}\n", info.authn_context));
        }
    }

    if !info.attributes.is_empty() {
        out.push_str("\n## Attributes\n\n");
        out.push_str(&format!("  {:<40} {}\n", "NAME", "VALUE(S)"));
        out.push_str(&format!("  {}\n", "-".repeat(70)));
        for (name, vals) in &info.attributes {
            let short_name = name.rsplit('/').next().unwrap_or(name);
            let display_val = vals.join(", ");
            let val_preview: String = display_val.chars().take(60).collect();
            let ellipsis = if display_val.len() > 60 { "..." } else { "" };
            out.push_str(&format!("  {:<40} {}{}\n", short_name, val_preview, ellipsis));
        }
    }

    Ok(out)
}

fn action_attributes(args: &Value) -> Result<String, String> {
    let xml = load_saml(args)?;
    let root = parse_xml_simple(&xml)?;
    let info = parse_saml_doc(&root);

    if info.encrypted_assertion {
        return Ok("Assertion is encrypted — cannot extract attributes without the SP private key.".to_string());
    }

    if info.attributes.is_empty() {
        return Ok("No attributes found in this SAML assertion.".to_string());
    }

    let mut out = format!("## SAML Attributes ({} total)\n\n", info.attributes.len());
    for (name, vals) in &info.attributes {
        out.push_str(&format!("### {}\n", name));
        for v in vals {
            out.push_str(&format!("  {}\n", v));
        }
        out.push('\n');
    }

    // Common attribute mapping hints
    out.push_str("## Common Attribute Mappings\n\n");
    let known: &[(&str, &str)] = &[
        ("NameID",             "Primary user identifier"),
        ("emailaddress",       "User email — often the login identifier"),
        ("givenname",          "First name"),
        ("surname",            "Last name"),
        ("name",               "Display name"),
        ("upn",                "User Principal Name (Active Directory UPN)"),
        ("groups",             "Group memberships — used for role-based access"),
        ("role",               "Application role assignment"),
        ("objectidentifier",   "Azure AD / Entra object ID (immutable)"),
        ("tenantid",           "Azure AD tenant ID"),
    ];
    let attrs_lower: Vec<String> = info.attributes.iter()
        .map(|(n, _)| n.rsplit('/').next().unwrap_or(n).to_lowercase())
        .collect();
    for (key, desc) in known {
        if attrs_lower.iter().any(|a| a.contains(key)) {
            out.push_str(&format!("  {:<25} {}\n", key, desc));
        }
    }

    Ok(out)
}

fn action_validate(args: &Value) -> Result<String, String> {
    let xml = load_saml(args)?;
    let root = parse_xml_simple(&xml)?;
    let info = parse_saml_doc(&root);

    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut ok_items: Vec<String> = Vec::new();

    // Status
    if !info.status_code.is_empty() {
        if info.status_code == "Success" {
            ok_items.push("Status: Success".to_string());
        } else {
            issues.push(format!("Status is not Success: {} — {}", info.status_code, info.status_message));
        }
    }

    // Signature
    if info.signature_present {
        ok_items.push("XML signature element present".to_string());
        warnings.push("Signature presence verified — but signature cryptographic validation requires SP private key and IdP certificate (cannot be done offline here)".to_string());
    } else {
        issues.push("No XML signature found — unsigned assertions are not trustworthy".to_string());
    }

    // Issuer
    if info.issuer.is_empty() {
        issues.push("Missing Issuer element".to_string());
    } else {
        ok_items.push(format!("Issuer present: {}", info.issuer));
    }

    // Subject
    if info.subject_nameid.is_empty() {
        if !info.encrypted_assertion {
            issues.push("No NameID found in Subject".to_string());
        }
    } else {
        ok_items.push(format!("NameID present: {}", &info.subject_nameid[..info.subject_nameid.len().min(60)]));
    }

    // Validity window
    if info.not_before.is_empty() && info.not_on_or_after.is_empty() {
        warnings.push("No Conditions element with validity window — no time constraint on this assertion".to_string());
    } else {
        ok_items.push(format!("Conditions window: {} to {}", info.not_before, info.not_on_or_after));
        // Note: actual time comparison requires current time; we flag it but can't check without chrono
        warnings.push("Time-based validity (NotBefore/NotOnOrAfter) not checked — verify against current UTC time manually".to_string());
    }

    // Encrypted assertion
    if info.encrypted_assertion {
        warnings.push("Assertion is encrypted — attribute inspection skipped".to_string());
    }

    // InResponseTo
    if info.in_response_to.is_empty() && info.doc_type == "Response" {
        warnings.push("No InResponseTo attribute — this is an IdP-initiated assertion (not SP-initiated); validate that this flow is expected".to_string());
    } else if !info.in_response_to.is_empty() {
        ok_items.push(format!("InResponseTo present: {} (verify against original AuthnRequest ID)", info.in_response_to));
    }

    // Destination
    if info.destination.is_empty() {
        warnings.push("No Destination attribute — SP should reject assertions without a matching Destination".to_string());
    } else {
        ok_items.push(format!("Destination: {}", info.destination));
    }

    let verdict = if !issues.is_empty() { "INVALID" } else if !warnings.is_empty() { "VALID (with warnings)" } else { "VALID" };

    let mut out = format!("## SAML Validation: {}\n\n", verdict);

    if !ok_items.is_empty() {
        out.push_str("## ✓ OK\n\n");
        for item in &ok_items { out.push_str(&format!("  ✓ {}\n", item)); }
        out.push('\n');
    }

    if !warnings.is_empty() {
        out.push_str("## ⚠ Warnings\n\n");
        for w in &warnings { out.push_str(&format!("  ⚠  {}\n", w)); }
        out.push('\n');
    }

    if !issues.is_empty() {
        out.push_str("## ✗ Issues\n\n");
        for issue in &issues { out.push_str(&format!("  ✗ {}\n", issue)); }
        out.push('\n');
    }

    out.push_str("## Security Checklist (manual steps required)\n\n");
    out.push_str("  1. Verify XML signature using IdP's public certificate\n");
    out.push_str("  2. Confirm Issuer matches the trusted IdP entity ID\n");
    out.push_str("  3. Confirm Destination matches your ACS URL\n");
    out.push_str("  4. Confirm InResponseTo matches your AuthnRequest ID (SP-initiated)\n");
    out.push_str("  5. Confirm current time is within NotBefore/NotOnOrAfter window\n");
    out.push_str("  6. Confirm Audience matches your SP entity ID in AudienceRestriction\n");
    out.push_str("  7. Replay protection: store and reject seen assertion IDs (use AssertionID cache)\n");

    Ok(out)
}

fn action_explain(args: &Value) -> Result<String, String> {
    let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("sso");

    let text = match topic {
        "bindings" => concat!(
            "## SAML 2.0 Bindings\n\n",
            "Bindings define how SAML messages are transported.\n\n",
            "  HTTP Redirect Binding\n",
            "    Used for AuthnRequests (SP → IdP). Message is deflate-compressed, base64-encoded,\n",
            "    and placed in the URL query string. URL-signed with a query-string signature.\n",
            "    Max message size limited by URL length (~8 KB). Most common for request initiation.\n\n",
            "  HTTP POST Binding\n",
            "    Used for Responses (IdP → SP). SAMLResponse is base64-encoded (not compressed)\n",
            "    and sent as an HTML form field via auto-submit JavaScript.\n",
            "    No message size limit. Standard for responses. Can carry large assertions.\n\n",
            "  HTTP Artifact Binding\n",
            "    SP or IdP sends only a short artifact reference. Recipient fetches the actual\n",
            "    message from the sender's ArtifactResolutionService via a back-channel SOAP call.\n",
            "    Prevents assertion exposure in browser history. Complex to implement.\n\n",
            "  SOAP Binding\n",
            "    Used for back-channel messages (ArtifactResolution, SLO). SOAP over HTTPS.\n",
        ),
        "assertions" => concat!(
            "## SAML 2.0 Assertion Structure\n\n",
            "  <samlp:Response>           — wrapper for IdP responses\n",
            "    <saml:Issuer>            — IdP entity ID\n",
            "    <samlp:Status>           — Success / Requester error / Responder error\n",
            "    <saml:Assertion>         — the security statement\n",
            "      <saml:Issuer>          — assertion issuer\n",
            "      <ds:Signature>         — XML digital signature\n",
            "      <saml:Subject>         — who the assertion is about\n",
            "        <saml:NameID>        — user identifier (email, upn, opaque ID, etc.)\n",
            "        <saml:SubjectConfirmation>\n",
            "          <saml:SubjectConfirmationData  NotOnOrAfter='..'  Recipient='..'  InResponseTo='..'/>\n",
            "      <saml:Conditions  NotBefore='..'  NotOnOrAfter='..'>\n",
            "        <saml:AudienceRestriction>\n",
            "          <saml:Audience>    — SP entity ID (must match)\n",
            "      <saml:AuthnStatement  SessionIndex='..'>  — authentication event\n",
            "        <saml:AuthnContext>\n",
            "          <saml:AuthnContextClassRef>  — e.g. PasswordProtectedTransport\n",
            "      <saml:AttributeStatement>  — user attributes\n",
            "        <saml:Attribute Name='..'>\n",
            "          <saml:AttributeValue>  value </saml:AttributeValue>\n",
        ),
        "idp" => concat!(
            "## SAML Identity Provider (IdP)\n\n",
            "The IdP authenticates users and issues SAML assertions.\n\n",
            "  Key IdP metadata fields:\n",
            "    entityID           Unique URI identifying the IdP\n",
            "    IDPSSODescriptor   IdP capabilities and endpoints\n",
            "    SingleSignOnService ACS/POST endpoint where SP sends AuthnRequests\n",
            "    SingleLogoutService SLO endpoint\n",
            "    KeyDescriptor      IdP signing certificate (sp must trust this)\n\n",
            "  Common enterprise IdPs:\n",
            "    Microsoft Entra ID (Azure AD)  — enterprise Microsoft 365 identities\n",
            "    Okta                           — cloud-first identity platform\n",
            "    Ping Identity / PingFederate   — enterprise federation\n",
            "    ADFS (Active Directory FS)     — on-premises Windows federation\n",
            "    Google Workspace               — Google/Gmail enterprise accounts\n",
            "    OneLogin                       — cloud identity management\n",
        ),
        "sp" => concat!(
            "## SAML Service Provider (SP)\n\n",
            "The SP is your application — it consumes SAML assertions from the IdP.\n\n",
            "  Key SP metadata fields:\n",
            "    entityID           Unique URI identifying your app\n",
            "    SPSSODescriptor    SP capabilities\n",
            "    AssertionConsumerService  Your ACS URL (where IdP posts responses)\n",
            "    SingleLogoutService SLO callback\n",
            "    KeyDescriptor      SP encryption certificate (for EncryptedAssertions)\n\n",
            "  SP-initiated SSO flow:\n",
            "    1. User hits protected resource → SP generates AuthnRequest\n",
            "    2. SP redirects user to IdP (HTTP Redirect binding)\n",
            "    3. IdP authenticates user\n",
            "    4. IdP POSTs SAMLResponse to SP ACS URL (HTTP POST binding)\n",
            "    5. SP validates assertion → creates session → redirects to resource\n\n",
            "  IdP-initiated SSO flow:\n",
            "    1. User logs into IdP portal and clicks app tile\n",
            "    2. IdP POSTs assertion without prior AuthnRequest (no InResponseTo)\n",
            "    3. SP validates and creates session\n",
            "    Note: IdP-initiated has weaker replay-attack protection\n",
        ),
        "security" => concat!(
            "## SAML Security Checklist\n\n",
            "  Signature validation:\n",
            "    ✓ Verify XML signature on the Assertion or Response element\n",
            "    ✓ Use IdP's certificate from their metadata — do not accept self-reported certs\n",
            "    ✓ Reject if signature is absent or invalid\n",
            "    ✓ Watch for XML signature wrapping (XSW) attacks — validate signed element ID\n\n",
            "  Time validation:\n",
            "    ✓ NotBefore <= now <= NotOnOrAfter\n",
            "    ✓ SubjectConfirmationData.NotOnOrAfter > now\n",
            "    ✓ Allow ±5 minute clock skew for distributed systems\n\n",
            "  Binding validation:\n",
            "    ✓ Destination must match your ACS URL exactly\n",
            "    ✓ Audience must match your SP entity ID exactly\n",
            "    ✓ InResponseTo must match a pending AuthnRequest ID (SP-initiated)\n\n",
            "  Replay protection:\n",
            "    ✓ Cache seen assertion IDs for the validity window duration\n",
            "    ✓ Reject duplicate assertion IDs\n\n",
            "  Issuer validation:\n",
            "    ✓ Issuer must match the configured trusted IdP entity ID\n\n",
            "  Common SAML attacks to defend against:\n",
            "    XSW (XML Signature Wrapping) — attacker injects a second assertion\n",
            "    Replay attack               — reuse of a valid assertion\n",
            "    CSRF on ACS endpoint        — browser-based POST binding forgery\n",
            "    NameID injection            — malformed NameID values\n",
        ),
        _ /* sso */ => concat!(
            "## SAML 2.0 SSO Overview\n\n",
            "SAML (Security Assertion Markup Language) is an XML-based standard for federated\n",
            "identity and Single Sign-On (SSO). Version 2.0 is the current standard (2005).\n\n",
            "  Key participants:\n",
            "    IdP (Identity Provider)   — authenticates users, issues assertions (Okta, Azure AD, ADFS)\n",
            "    SP (Service Provider)     — your application, trusts the IdP's assertions\n",
            "    User/Browser              — the human navigating between IdP and SP\n\n",
            "  Core documents:\n",
            "    AuthnRequest   SP → IdP: 'please authenticate this user'\n",
            "    SAMLResponse   IdP → SP: 'here is a signed assertion about the user'\n",
            "    Assertion      The security statement inside SAMLResponse\n",
            "    Metadata       Machine-readable configuration document for either party\n\n",
            "  vs OAuth 2.0 / OIDC:\n",
            "    SAML   XML-based, enterprise-centric, browser redirect/POST, heavy tooling\n",
            "    OIDC   JSON/JWT-based, developer-friendly, mobile/SPA/API support\n",
            "    Both achieve SSO — choice depends on your IdP and protocol support\n\n",
            "Use topic='bindings', 'assertions', 'idp', 'sp', or 'security' for detail.\n",
        ),
    };

    Ok(text.to_string())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or_else(|| {
        if args.get("topic").is_some() { "explain" }
        else { "parse" }
    });
    match action {
        "attributes" => action_attributes(args),
        "validate"   => action_validate(args),
        "explain"    => action_explain(args),
        _            => action_parse(args),
    }
}
