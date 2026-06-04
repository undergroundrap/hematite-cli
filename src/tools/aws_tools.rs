use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["arn", "s3", "region", "service"],
                "description": "Action: arn (default), s3, region, service"
            },
            "input": { "type": "string", "description": "ARN, S3 URI/URL, region code, or service name/code to look up" },
            "query": { "type": "string", "description": "Search query for region or service lookup" }
        }
    })
}

struct ArnParts {
    partition: String,
    service: String,
    region: String,
    account: String,
    resource: String,
}

fn parse_arn(arn: &str) -> Result<ArnParts, String> {
    let s = arn.trim();
    if !s.starts_with("arn:") {
        return Err(format!(
            "Not an ARN — must start with 'arn:' (got '{}')",
            &s[..s.len().min(20)]
        ));
    }
    let parts: Vec<&str> = s.splitn(6, ':').collect();
    if parts.len() < 6 {
        return Err(format!("Malformed ARN — expected arn:partition:service:region:account:resource, got {} components", parts.len()));
    }
    Ok(ArnParts {
        partition: parts[1].to_string(),
        service: parts[2].to_string(),
        region: parts[3].to_string(),
        account: parts[4].to_string(),
        resource: parts[5].to_string(),
    })
}

struct RegionEntry {
    code: &'static str,
    name: &'static str,
    geo: &'static str,
}

static REGIONS: &[RegionEntry] = &[
    RegionEntry {
        code: "us-east-1",
        name: "US East (N. Virginia)",
        geo: "North America",
    },
    RegionEntry {
        code: "us-east-2",
        name: "US East (Ohio)",
        geo: "North America",
    },
    RegionEntry {
        code: "us-west-1",
        name: "US West (N. California)",
        geo: "North America",
    },
    RegionEntry {
        code: "us-west-2",
        name: "US West (Oregon)",
        geo: "North America",
    },
    RegionEntry {
        code: "ca-central-1",
        name: "Canada (Central)",
        geo: "North America",
    },
    RegionEntry {
        code: "ca-west-1",
        name: "Canada West (Calgary)",
        geo: "North America",
    },
    RegionEntry {
        code: "eu-west-1",
        name: "Europe (Ireland)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-west-2",
        name: "Europe (London)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-west-3",
        name: "Europe (Paris)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-central-1",
        name: "Europe (Frankfurt)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-central-2",
        name: "Europe (Zurich)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-north-1",
        name: "Europe (Stockholm)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-south-1",
        name: "Europe (Milan)",
        geo: "Europe",
    },
    RegionEntry {
        code: "eu-south-2",
        name: "Europe (Spain)",
        geo: "Europe",
    },
    RegionEntry {
        code: "ap-east-1",
        name: "Asia Pacific (Hong Kong)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-south-1",
        name: "Asia Pacific (Mumbai)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-south-2",
        name: "Asia Pacific (Hyderabad)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-southeast-1",
        name: "Asia Pacific (Singapore)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-southeast-2",
        name: "Asia Pacific (Sydney)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-southeast-3",
        name: "Asia Pacific (Jakarta)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-southeast-4",
        name: "Asia Pacific (Melbourne)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-northeast-1",
        name: "Asia Pacific (Tokyo)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-northeast-2",
        name: "Asia Pacific (Seoul)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "ap-northeast-3",
        name: "Asia Pacific (Osaka)",
        geo: "Asia Pacific",
    },
    RegionEntry {
        code: "me-south-1",
        name: "Middle East (Bahrain)",
        geo: "Middle East",
    },
    RegionEntry {
        code: "me-central-1",
        name: "Middle East (UAE)",
        geo: "Middle East",
    },
    RegionEntry {
        code: "il-central-1",
        name: "Israel (Tel Aviv)",
        geo: "Middle East",
    },
    RegionEntry {
        code: "af-south-1",
        name: "Africa (Cape Town)",
        geo: "Africa",
    },
    RegionEntry {
        code: "sa-east-1",
        name: "South America (São Paulo)",
        geo: "South America",
    },
    RegionEntry {
        code: "us-gov-east-1",
        name: "AWS GovCloud (US-East)",
        geo: "GovCloud",
    },
    RegionEntry {
        code: "us-gov-west-1",
        name: "AWS GovCloud (US-West)",
        geo: "GovCloud",
    },
    RegionEntry {
        code: "cn-north-1",
        name: "China (Beijing)",
        geo: "China",
    },
    RegionEntry {
        code: "cn-northwest-1",
        name: "China (Ningxia)",
        geo: "China",
    },
];

struct ServiceEntry {
    code: &'static str,
    name: &'static str,
    category: &'static str,
}

static SERVICES: &[ServiceEntry] = &[
    ServiceEntry {
        code: "s3",
        name: "Simple Storage Service",
        category: "Storage",
    },
    ServiceEntry {
        code: "ec2",
        name: "Elastic Compute Cloud",
        category: "Compute",
    },
    ServiceEntry {
        code: "lambda",
        name: "Lambda",
        category: "Compute",
    },
    ServiceEntry {
        code: "ecs",
        name: "Elastic Container Service",
        category: "Containers",
    },
    ServiceEntry {
        code: "eks",
        name: "Elastic Kubernetes Service",
        category: "Containers",
    },
    ServiceEntry {
        code: "ecr",
        name: "Elastic Container Registry",
        category: "Containers",
    },
    ServiceEntry {
        code: "rds",
        name: "Relational Database Service",
        category: "Database",
    },
    ServiceEntry {
        code: "dynamodb",
        name: "DynamoDB",
        category: "Database",
    },
    ServiceEntry {
        code: "elasticache",
        name: "ElastiCache",
        category: "Database",
    },
    ServiceEntry {
        code: "redshift",
        name: "Redshift",
        category: "Analytics",
    },
    ServiceEntry {
        code: "athena",
        name: "Athena",
        category: "Analytics",
    },
    ServiceEntry {
        code: "glue",
        name: "Glue",
        category: "Analytics",
    },
    ServiceEntry {
        code: "kinesis",
        name: "Kinesis",
        category: "Analytics",
    },
    ServiceEntry {
        code: "sqs",
        name: "Simple Queue Service",
        category: "Messaging",
    },
    ServiceEntry {
        code: "sns",
        name: "Simple Notification Service",
        category: "Messaging",
    },
    ServiceEntry {
        code: "ses",
        name: "Simple Email Service",
        category: "Messaging",
    },
    ServiceEntry {
        code: "eventbridge",
        name: "EventBridge",
        category: "Messaging",
    },
    ServiceEntry {
        code: "iam",
        name: "Identity and Access Management",
        category: "Security",
    },
    ServiceEntry {
        code: "sts",
        name: "Security Token Service",
        category: "Security",
    },
    ServiceEntry {
        code: "kms",
        name: "Key Management Service",
        category: "Security",
    },
    ServiceEntry {
        code: "secretsmanager",
        name: "Secrets Manager",
        category: "Security",
    },
    ServiceEntry {
        code: "waf",
        name: "Web Application Firewall",
        category: "Security",
    },
    ServiceEntry {
        code: "guardduty",
        name: "GuardDuty",
        category: "Security",
    },
    ServiceEntry {
        code: "cloudfront",
        name: "CloudFront",
        category: "CDN/Network",
    },
    ServiceEntry {
        code: "route53",
        name: "Route 53",
        category: "CDN/Network",
    },
    ServiceEntry {
        code: "vpc",
        name: "Virtual Private Cloud",
        category: "CDN/Network",
    },
    ServiceEntry {
        code: "elb",
        name: "Elastic Load Balancing",
        category: "CDN/Network",
    },
    ServiceEntry {
        code: "apigateway",
        name: "API Gateway",
        category: "CDN/Network",
    },
    ServiceEntry {
        code: "cloudwatch",
        name: "CloudWatch",
        category: "Monitoring",
    },
    ServiceEntry {
        code: "cloudtrail",
        name: "CloudTrail",
        category: "Monitoring",
    },
    ServiceEntry {
        code: "config",
        name: "Config",
        category: "Monitoring",
    },
    ServiceEntry {
        code: "xray",
        name: "X-Ray",
        category: "Monitoring",
    },
    ServiceEntry {
        code: "cloudformation",
        name: "CloudFormation",
        category: "Ops",
    },
    ServiceEntry {
        code: "codepipeline",
        name: "CodePipeline",
        category: "Ops",
    },
    ServiceEntry {
        code: "codebuild",
        name: "CodeBuild",
        category: "Ops",
    },
    ServiceEntry {
        code: "codecommit",
        name: "CodeCommit",
        category: "Ops",
    },
    ServiceEntry {
        code: "codedeploy",
        name: "CodeDeploy",
        category: "Ops",
    },
    ServiceEntry {
        code: "ssm",
        name: "Systems Manager",
        category: "Ops",
    },
    ServiceEntry {
        code: "backup",
        name: "Backup",
        category: "Ops",
    },
    ServiceEntry {
        code: "bedrock",
        name: "Bedrock",
        category: "AI/ML",
    },
    ServiceEntry {
        code: "sagemaker",
        name: "SageMaker",
        category: "AI/ML",
    },
    ServiceEntry {
        code: "rekognition",
        name: "Rekognition",
        category: "AI/ML",
    },
    ServiceEntry {
        code: "transcribe",
        name: "Transcribe",
        category: "AI/ML",
    },
    ServiceEntry {
        code: "translate",
        name: "Translate",
        category: "AI/ML",
    },
];

fn lookup_region(code: &str) -> Option<&'static RegionEntry> {
    REGIONS.iter().find(|r| r.code.eq_ignore_ascii_case(code))
}

fn lookup_service(code: &str) -> Option<&'static ServiceEntry> {
    SERVICES.iter().find(|s| s.code.eq_ignore_ascii_case(code))
}

fn service_label(svc_code: &str) -> String {
    if let Some(s) = lookup_service(svc_code) {
        format!("{} ({})", s.name, s.category)
    } else {
        svc_code.to_string()
    }
}

fn resource_type_hint(service: &str, resource: &str) -> String {
    let res = resource.to_lowercase();
    match service.to_lowercase().as_str() {
        "s3" => {
            if res.contains('/') {
                "S3 object".to_string()
            } else {
                "S3 bucket".to_string()
            }
        }
        "ec2" => {
            if res.starts_with("instance/") {
                "EC2 instance".to_string()
            } else if res.starts_with("security-group/") {
                "Security group".to_string()
            } else if res.starts_with("subnet/") {
                "Subnet".to_string()
            } else if res.starts_with("vpc/") {
                "VPC".to_string()
            } else if res.starts_with("volume/") {
                "EBS volume".to_string()
            } else {
                "EC2 resource".to_string()
            }
        }
        "iam" => {
            if res.starts_with("role/") {
                "IAM role".to_string()
            } else if res.starts_with("user/") {
                "IAM user".to_string()
            } else if res.starts_with("group/") {
                "IAM group".to_string()
            } else if res.starts_with("policy/") {
                "IAM policy".to_string()
            } else {
                "IAM resource".to_string()
            }
        }
        "lambda" => "Lambda function".to_string(),
        "rds" => {
            if res.starts_with("db:") {
                "RDS database instance".to_string()
            } else if res.starts_with("cluster:") {
                "RDS cluster".to_string()
            } else {
                "RDS resource".to_string()
            }
        }
        "dynamodb" => {
            if res.starts_with("table/") {
                "DynamoDB table".to_string()
            } else {
                "DynamoDB resource".to_string()
            }
        }
        "sqs" => "SQS queue".to_string(),
        "sns" => "SNS topic".to_string(),
        "kms" => "KMS key".to_string(),
        "secretsmanager" => "Secret".to_string(),
        _ => format!("{} resource", service),
    }
}

fn action_arn(args: &Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'input' with an ARN string (e.g. arn:aws:s3:::my-bucket)")?;

    let arn = parse_arn(input)?;

    let region_line = if arn.region.is_empty() {
        "  Region:    (global resource)".to_string()
    } else {
        match lookup_region(&arn.region) {
            Some(r) => format!("  Region:    {} — {}", arn.region, r.name),
            None => format!("  Region:    {} (unrecognized)", arn.region),
        }
    };

    let account_line = if arn.account.is_empty() {
        "  Account:   (no account)".to_string()
    } else {
        format!("  Account:   {}", arn.account)
    };

    let resource_hint = resource_type_hint(&arn.service, &arn.resource);

    let mut out = String::new();
    out.push_str("## ARN Breakdown\n\n");
    out.push_str(&format!("  ARN:       {}\n\n", input));
    out.push_str(&format!("  Partition: {}\n", arn.partition));
    out.push_str(&format!(
        "  Service:   {} — {}\n",
        arn.service,
        service_label(&arn.service)
    ));
    out.push_str(&region_line);
    out.push('\n');
    out.push_str(&account_line);
    out.push('\n');
    out.push_str(&format!("  Resource:  {}\n", arn.resource));
    out.push_str(&format!("  Type hint: {}\n", resource_hint));

    // Partition notes
    out.push_str("\n## Notes\n\n");
    match arn.partition.as_str() {
        "aws" => out.push_str("  Standard commercial partition (aws).\n"),
        "aws-cn" => {
            out.push_str("  China partition (aws-cn) — separate from commercial regions.\n")
        }
        "aws-us-gov" => {
            out.push_str("  GovCloud partition (aws-us-gov) — restricted to US government.\n")
        }
        other => out.push_str(&format!("  Non-standard partition: {}\n", other)),
    }
    if arn.account.is_empty() {
        out.push_str("  No account ID — this is a global/shared resource (e.g. S3 bucket ARN).\n");
    }
    if arn.region.is_empty() {
        out.push_str("  No region — this is a global service (e.g. IAM, Route 53).\n");
    }

    Ok(out)
}

fn parse_s3_uri(input: &str) -> Result<(String, String), String> {
    let s = input.trim();
    if let Some(rest) = s.strip_prefix("s3://") {
        let (bucket, key) = match rest.find('/') {
            Some(i) => (rest[..i].to_string(), rest[i + 1..].to_string()),
            None => (rest.to_string(), String::new()),
        };
        return Ok((bucket, key));
    }
    // https://bucket.s3.region.amazonaws.com/key or https://s3.region.amazonaws.com/bucket/key
    if s.starts_with("http://") || s.starts_with("https://") {
        let after_scheme = s.split("://").nth(1).unwrap_or(s);
        let (host_part, path_part) = match after_scheme.find('/') {
            Some(i) => (&after_scheme[..i], &after_scheme[i + 1..]),
            None => (after_scheme, ""),
        };
        let host = host_part.to_lowercase();
        if host.ends_with(".amazonaws.com") {
            // path-style: s3.region.amazonaws.com/bucket/key
            if host.starts_with("s3.") || host.starts_with("s3-") {
                let (bucket, key) = match path_part.find('/') {
                    Some(i) => (path_part[..i].to_string(), path_part[i + 1..].to_string()),
                    None => (path_part.to_string(), String::new()),
                };
                return Ok((bucket, key));
            }
            // virtual-hosted: bucket.s3.region.amazonaws.com/key
            if let Some(dot_pos) = host.find(".s3") {
                let bucket = host_part[..dot_pos].to_string();
                let key = path_part.to_string();
                return Ok((bucket, key));
            }
        }
    }
    Err(format!(
        "Unrecognized S3 URI/URL format: '{}'",
        &s[..s.len().min(60)]
    ))
}

fn action_s3(args: &Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'input' with an S3 URI (s3://bucket/key) or HTTPS URL")?;

    let (bucket, key) = parse_s3_uri(input)?;

    let mut out = String::new();
    out.push_str("## S3 URI Breakdown\n\n");
    out.push_str(&format!("  Input:    {}\n\n", input));
    out.push_str(&format!("  Bucket:   {}\n", bucket));
    if key.is_empty() {
        out.push_str("  Key:      (root/bucket-level — no object key)\n");
    } else {
        let ext = std::path::Path::new(&key)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        out.push_str(&format!("  Key:      {}\n", key));
        if !ext.is_empty() {
            out.push_str(&format!("  Ext:      .{}\n", ext));
        }
        // Depth
        let depth = key.chars().filter(|&c| c == '/').count();
        if depth > 0 {
            out.push_str(&format!("  Depth:    {} prefix segment(s)\n", depth));
        }
    }

    out.push_str("\n## Equivalent Formats\n\n");
    out.push_str(&format!("  S3 URI:       s3://{}/{}\n", bucket, key));
    out.push_str(&format!(
        "  Virtual-host: https://{}.s3.amazonaws.com/{}\n",
        bucket, key
    ));
    out.push_str(&format!(
        "  Path-style:   https://s3.amazonaws.com/{}/{}\n",
        bucket, key
    ));
    out.push_str(&format!(
        "  ARN:          arn:aws:s3:::{}/{}\n",
        bucket, key
    ));
    out.push_str(&format!(
        "  Console URL:  https://s3.console.aws.amazon.com/s3/object/{}/{}\n",
        bucket, key
    ));

    Ok(out)
}

fn action_region(args: &Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str());

    if let Some(q) = input {
        let q_lower = q.to_lowercase();
        // Exact code match
        if let Some(r) = lookup_region(q) {
            let mut out = format!("## Region: {}\n\n", r.code);
            out.push_str(&format!("  Code:      {}\n", r.code));
            out.push_str(&format!("  Name:      {}\n", r.name));
            out.push_str(&format!("  Geography: {}\n", r.geo));
            out.push_str(&format!("  S3 bucket: s3.{}.amazonaws.com\n", r.code));
            out.push_str(&format!(
                "  Endpoint:  https://ec2.{}.amazonaws.com\n",
                r.code
            ));
            return Ok(out);
        }
        // Substring search
        let matches: Vec<&RegionEntry> = REGIONS
            .iter()
            .filter(|r| {
                r.code.contains(&q_lower)
                    || r.name.to_lowercase().contains(&q_lower)
                    || r.geo.to_lowercase().contains(&q_lower)
            })
            .collect();
        if matches.is_empty() {
            return Ok(format!("No regions found matching '{}'.\n\nUse action='region' without input to list all regions.", q));
        }
        let mut out = format!("## Regions matching '{}'\n\n", q);
        out.push_str(&format!("{:<24} {:<36} {}\n", "CODE", "NAME", "GEO"));
        out.push_str(&format!("{}\n", "-".repeat(70)));
        for r in matches {
            out.push_str(&format!("{:<24} {:<36} {}\n", r.code, r.name, r.geo));
        }
        return Ok(out);
    }

    // List all
    let mut out = format!("{:<24} {:<36} {}\n", "CODE", "NAME", "GEO");
    out.push_str(&format!("{}\n", "-".repeat(70)));
    let mut current_geo = "";
    for r in REGIONS {
        if r.geo != current_geo {
            current_geo = r.geo;
        }
        out.push_str(&format!("{:<24} {:<36} {}\n", r.code, r.name, r.geo));
    }
    out.push_str(&format!("\nTotal: {} regions\n", REGIONS.len()));
    Ok(out)
}

fn action_service(args: &Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str());

    if let Some(q) = input {
        let q_lower = q.to_lowercase();
        if let Some(s) = lookup_service(q) {
            let mut out = format!("## AWS Service: {}\n\n", s.code);
            out.push_str(&format!("  Service code: {}\n", s.code));
            out.push_str(&format!("  Full name:    {}\n", s.name));
            out.push_str(&format!("  Category:     {}\n", s.category));
            out.push_str(&format!(
                "  ARN prefix:   arn:aws:{}:REGION:ACCOUNT:RESOURCE\n",
                s.code
            ));
            return Ok(out);
        }
        let matches: Vec<&ServiceEntry> = SERVICES
            .iter()
            .filter(|s| {
                s.code.contains(&q_lower)
                    || s.name.to_lowercase().contains(&q_lower)
                    || s.category.to_lowercase().contains(&q_lower)
            })
            .collect();
        if matches.is_empty() {
            return Ok(format!("No services found matching '{}'.", q));
        }
        let mut out = format!("## AWS Services matching '{}'\n\n", q);
        out.push_str(&format!("{:<20} {:<40} {}\n", "CODE", "NAME", "CATEGORY"));
        out.push_str(&format!("{}\n", "-".repeat(72)));
        for s in matches {
            out.push_str(&format!("{:<20} {:<40} {}\n", s.code, s.name, s.category));
        }
        return Ok(out);
    }

    let mut out = format!("{:<20} {:<40} {}\n", "CODE", "NAME", "CATEGORY");
    out.push_str(&format!("{}\n", "-".repeat(72)));
    for s in SERVICES {
        out.push_str(&format!("{:<20} {:<40} {}\n", s.code, s.name, s.category));
    }
    out.push_str(&format!("\nTotal: {} services\n", SERVICES.len()));
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            if input.starts_with("arn:") {
                "arn"
            } else if input.starts_with("s3://")
                || input.contains("s3.amazonaws.com")
                || input.contains(".s3.")
            {
                "s3"
            } else {
                "arn"
            }
        });
    match action {
        "s3" => action_s3(args),
        "region" => action_region(args),
        "service" => action_service(args),
        _ => action_arn(args),
    }
}
