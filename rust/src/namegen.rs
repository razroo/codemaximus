//! Shared word pools and RNG helpers (mirrors ``naming`` / ``comments``).

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub const CORP_PREFIXES: &[&str] = &[
    "Abstract", "Base", "Default", "Internal", "Core", "Enhanced", "Optimized",
    "Legacy", "Modern", "Enterprise", "Cloud", "Distributed", "Scalable",
    "Dynamic", "Static", "Global", "Local", "Custom", "Generic", "Standard",
];

pub const CORP_NOUNS: &[&str] = &[
    "Singleton", "Factory", "Builder", "Proxy", "Adapter", "Decorator",
    "Observer", "Strategy", "Command", "Mediator", "Facade", "Bridge",
    "Composite", "Iterator", "Visitor", "Prototype", "Flyweight", "Chain",
    "Handler", "Service", "Manager", "Controller", "Repository", "Provider",
    "Orchestrator", "Coordinator", "Validator", "Transformer", "Resolver",
    "Dispatcher", "Interceptor", "Aggregator", "Processor", "Pipeline",
    "Delegate", "Registry", "Configurator", "Initializer", "Serializer",
    "Deserializer", "Converter", "Mapper", "Wrapper", "Connector",
    "Bean", "Module", "Component", "Endpoint", "Gateway", "Middleware",
];

pub const CORP_SUFFIXES: &[&str] = &[
    "Impl", "Base", "Abstract", "Interface", "Helper", "Util", "Utils",
    "Config", "Context", "State", "Result", "Response", "Request",
    "Exception", "Error", "Type", "Kind", "Spec", "Definition", "Descriptor",
    "Info", "Data", "Model", "Entity", "Record", "Value", "Pair",
];

pub const CHAOS_WORDS: &[&str] = &[
    "Skibidi", "Ligma", "Yeet", "Bruh", "Sigma", "Rizz", "Gyatt",
    "Bussin", "NoCap", "Sus", "Slay", "Vibe", "Ohio", "Fanum",
    "Griddy", "Mewing", "Edging", "Gooning", "Aura", "Delulu",
    "Sussy", "Baka", "Poggers", "Copium", "Hopium", "Malding",
    "Gigachad", "Based", "Cringe", "Ratio", "Deadass", "Noob",
    "Chungus", "Dank", "Stonks", "Bonk", "Oof", "Yoink", "Sheesh",
    "Drip", "Glizzy", "Goated", "Slaps", "Hits", "Bussin",
    "xX_Destroyer_Xx", "L_plus_ratio", "skill_issue", "no_bitches",
];

pub const CORP_METHODS: &[&str] = &[
    "process", "handle", "execute", "validate", "transform", "convert",
    "initialize", "configure", "register", "resolve", "dispatch", "notify",
    "aggregate", "compute", "evaluate", "serialize", "deserialize", "parse",
    "render", "build", "create", "destroy", "update", "delete", "fetch",
    "load", "save", "persist", "cache", "invalidate", "refresh", "sync",
    "authenticate", "authorize", "encrypt", "decrypt", "compress", "decompress",
    "normalize", "denormalize", "sanitize", "format", "marshal", "unmarshal",
];

pub const CHAOS_METHODS: &[&str] = &[
    "yeet", "yoink", "vibe_check", "rizz_up", "no_cap", "bussin_fr",
    "do_the_thing", "idk_what_this_does", "please_work", "trust_me_bro",
    "todo_fix_later", "hack_around_it", "ship_it", "lgtm", "works_on_my_machine",
    "dont_touch_this", "here_be_dragons", "abandon_all_hope", "cry",
    "pray_to_the_machine_spirit", "sacrifice_to_the_compiler", "cope",
    "seethe", "mald", "touch_grass", "go_outside",
];

pub const CORP_VARS: &[&str] = &[
    "result", "value", "data", "response", "request", "context", "config",
    "state", "status", "count", "index", "item", "element", "node",
    "entity", "record", "entry", "instance", "reference", "target",
    "source", "destination", "input_data", "output_data", "buffer",
    "cache_entry", "metadata", "payload", "params", "options", "settings",
];

pub const CHAOS_VARS: &[&str] = &[
    "thingy", "stuff", "whatever", "idk", "bruh", "x", "xx", "xxx",
    "temp_but_permanent", "this_shouldnt_work", "magic_number",
    "dont_ask", "legacy_pain", "tech_debt", "god_object", "spaghetti",
    "yolo_var", "fix_me_please", "cursed_value", "forbidden_knowledge",
    "the_darkness", "eldritch_data", "haunted_reference", "it_lives",
];

pub const CORP_COMMENTS: &[&str] = &[
    "This method handles the core business logic for the enterprise workflow.",
    "DO NOT MODIFY - This is load-bearing architecture.",
    "Implements the AbstractFactory pattern for maximum extensibility.",
    "This class follows the Single Responsibility Principle (it has one responsibility: being enormous).",
    "TODO: Refactor this in Q3 (written in 2019).",
    "Per the architecture review board decision ARB-2847.",
    "This satisfies requirement REQ-ENTERPRISE-4392.",
    "Optimized for enterprise-grade throughput.",
    "Thread-safe implementation using the double-checked locking pattern.",
    "This is a critical path component - do not remove without VP approval.",
    "Legacy code - here be dragons.",
    "Reviewed and approved by the Technical Steering Committee.",
    "This abstraction layer provides necessary indirection for future scalability.",
    "Conforms to ISO 27001 compliance requirements.",
    "Part of the microservice decomposition initiative (Phase 7 of 12).",
    "This was the simplest solution after 6 months of design review.",
    "The previous implementation was 3 lines but didn't meet enterprise standards.",
];

pub const CHAOS_COMMENTS: &[&str] = &[
    "i dont know what this does but removing it breaks everything",
    "TODO: figure out why this works",
    "written at 3am, mass forgive me",
    "if you're reading this, turn back now",
    "this function is cursed",
    "DO NOT TOUCH - last person who modified this quit",
    r"¯\_(ツ)_/¯",
    "works on my machine ™",
    "the compiler demanded a blood sacrifice and this was it",
    "i asked chatgpt to write this and even it said no",
    "no tests needed, it's perfect (copium)",
    "this is load-bearing spaghetti",
    "abandon all hope ye who enter here",
    "the code is documentation enough (it is not)",
    "if this breaks, blame the intern (there is no intern)",
    "certified bruh moment",
    "vibe coded, do not question",
    "i will mass NOT be explaining this in the PR",
    "skill issue if you can't read this",
    "past me was a different person and i dont trust them",
    "this violates at least 3 design patterns and invents 2 new ones",
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "the mass of code grows. it hungers. it consumes.",
];

pub const CORP_DOCSTRINGS: &[&str] = &[
    "Initializes the {name} with the specified configuration parameters.",
    "Processes the incoming request through the validation pipeline.",
    "Transforms the input data according to the business rules engine.",
    "Orchestrates the workflow execution across distributed service boundaries.",
    "Delegates to the underlying implementation for concrete behavior.",
    "Resolves dependencies through the inversion of control container.",
    "Validates the state transition according to the finite state machine definition.",
];

pub const CHAOS_DOCSTRINGS: &[&str] = &[
    "dont ask me what this does because i genuinely do not know",
    "this function exists because someone said 'just add a wrapper'",
    "returns something. probably.",
    "args: stuff. returns: other stuff. raises: your blood pressure.",
    "TL;DR: it do be doing things tho",
    "side effects: may cause existential dread",
    "complexity: O(vibes)",
    "deprecated since mass birth but still called in 47 places",
];

pub const WRAPPER_LAYERS: &[&str] = &[
    "",
    "Internal",
    "InternalImpl",
    "InternalImplV2",
    "InternalImplV2Final",
    "InternalImplV2FinalFinal",
    "InternalImplV2FinalFinalForReal",
    "InternalImplV2FinalFinalForRealThisTime",
];

pub const ASSERTIONS: &[&str] = &[
    "self.assertTrue(True)",
    "self.assertFalse(False)",
    "self.assertEqual(1, 1)",
    "self.assertIsNone(None)",
    "self.assertIsNotNone(object())",
    "self.assertIn(1, [1, 2, 3])",
    "self.assertEqual('a', 'a')",
    "self.assertGreater(2, 1)",
    "self.assertLess(1, 2)",
];

pub const STDLIB_IMPORTS: &[&str] = &[
    "from abc import ABC, abstractmethod",
    "from typing import Any, Optional, Union, Protocol, TypeVar, Generic",
    "from dataclasses import dataclass, field",
    "from functools import wraps, lru_cache",
    "from enum import Enum, auto",
    "import logging",
    "import sys",
    "import os",
    "from collections import defaultdict",
    "from contextlib import contextmanager",
];

pub const ENUM_STATUS_VALUES: &[&str] = &[
    "PENDING",
    "ACTIVE",
    "PROCESSING",
    "VALIDATING",
    "TRANSFORMING",
    "ORCHESTRATING",
    "DELEGATING",
    "RESOLVING",
    "FINALIZING",
    "COMPLETED",
    "FAILED",
    "RETRYING",
    "CANCELLED",
    "DEPRECATED",
    "UNKNOWN",
    "ASCENDING",
    "TRANSCENDING",
    "VIBING",
    "EXISTING",
];

pub const TS_PRIMITIVES: &[&str] = &[
    "string",
    "number",
    "boolean",
    "any",
    "unknown",
    "never",
    "void",
    "null | undefined",
];

pub const GO_IMPORTS: &[&str] = &[
    "\"fmt\"",
    "\"errors\"",
    "\"context\"",
    "\"sync\"",
    "\"time\"",
    "\"strings\"",
    "\"strconv\"",
    "\"io\"",
    "\"os\"",
    "\"log\"",
    "\"encoding/json\"",
    "\"net/http\"",
    "\"database/sql\"",
    "\"crypto/rand\"",
    "\"math/big\"",
    "\"bytes\"",
];

pub const JAVA_TYPES: &[&str] = &[
    "String",
    "int",
    "long",
    "boolean",
    "double",
    "Object",
    "List<Object>",
    "Map<String, Object>",
    "Optional<String>",
    "AbstractFactory",
    "ServiceProvider",
    "CompletableFuture<Void>",
];

pub const GO_TYPES: &[&str] = &[
    "interface{}",
    "string",
    "int",
    "int64",
    "bool",
    "float64",
    "[]byte",
    "map[string]interface{}",
    "chan struct{}",
    "context.Context",
    "*sync.Mutex",
    "error",
    "func() error",
    "[]interface{}",
];

pub fn mix_seed(sanity: f64, file_index: i64) -> u64 {
    let s = sanity.to_bits();
    let f = file_index as u64;
    f.wrapping_mul(0x9E3779B97F4A7C15) ^ s
}

/// XOR salt so different language emitters diverge for the same index.
pub fn mix_seed_salted(sanity: f64, file_index: i64, salt: u64) -> u64 {
    mix_seed(sanity, file_index) ^ salt
}

pub fn new_rng(sanity: f64, file_index: i64, salt: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(mix_seed_salted(sanity, file_index, salt))
}

pub fn blend_choice<'a, R: Rng>(rng: &mut R, sanity: f64, corp: &'a [&str], chaos: &'a [&str]) -> &'a str {
    if rng.gen_bool(sanity.clamp(0.0, 1.0)) {
        corp.choose(rng).unwrap()
    } else {
        chaos.choose(rng).unwrap()
    }
}

pub fn comment<R: Rng>(rng: &mut R, sanity: f64) -> String {
    blend_choice(rng, sanity, CORP_COMMENTS, CHAOS_COMMENTS).to_string()
}

pub fn docstring<R: Rng>(rng: &mut R, sanity: f64, name: &str) -> String {
    if rng.gen_bool(sanity.clamp(0.0, 1.0)) {
        let t = CORP_DOCSTRINGS.choose(rng).unwrap();
        t.replace("{name}", name)
    } else {
        CHAOS_DOCSTRINGS.choose(rng).unwrap().to_string()
    }
}

pub fn class_name<R: Rng>(rng: &mut R, sanity: f64) -> String {
    let s = sanity.clamp(0.0, 1.0);
    let mut parts = Vec::new();
    let num_parts = if s > 0.5 {
        rng.gen_range(2..=4)
    } else {
        rng.gen_range(1..=3)
    };
    if rng.gen_bool(s) {
        parts.push(*CORP_PREFIXES.choose(rng).unwrap());
    }
    for _ in 0..num_parts {
        parts.push(blend_choice(rng, s, CORP_NOUNS, CHAOS_WORDS));
    }
    if rng.gen_bool(s * 0.7) {
        parts.push(*CORP_SUFFIXES.choose(rng).unwrap());
    }
    parts.concat()
}

pub fn method_name<R: Rng>(rng: &mut R, sanity: f64) -> String {
    blend_choice(rng, sanity, CORP_METHODS, CHAOS_METHODS).to_string()
}

pub fn var_name<R: Rng>(rng: &mut R, sanity: f64) -> String {
    blend_choice(rng, sanity, CORP_VARS, CHAOS_VARS).to_string()
}

pub fn java_package<R: Rng>(rng: &mut R, sanity: f64) -> String {
    if sanity > 0.5 {
        let domains = ["com", "org", "io", "net"];
        let companies = ["enterprise", "megacorp", "synergy", "cloudscale", "dataflow"];
        let modules = ["core", "service", "util", "framework", "platform", "engine"];
        format!(
            "{}.{}.{}",
            domains.choose(rng).unwrap(),
            companies.choose(rng).unwrap(),
            modules.choose(rng).unwrap()
        )
    } else {
        format!(
            "com.{}.{}",
            CHAOS_WORDS.choose(rng).unwrap().to_lowercase(),
            CHAOS_WORDS.choose(rng).unwrap().to_lowercase()
        )
    }
}

pub fn go_package<R: Rng>(rng: &mut R, sanity: f64) -> String {
    if sanity > 0.5 {
        [
            "service", "handler", "middleware", "repository", "controller", "util",
        ]
        .choose(rng)
        .unwrap()
        .to_string()
    } else {
        ["yeet", "bruh", "skibidi", "sus", "ohio", "rizz"]
            .choose(rng)
            .unwrap()
            .to_string()
    }
}

pub fn author_line<R: Rng>(rng: &mut R, sanity: f64) -> String {
    let corp = [
        "Enterprise Code Generator",
        "Architecture Team",
        "Senior Staff Engineer",
    ];
    let chaos = ["a mass of vibes", "the mass void", "nobody"];
    blend_choice(rng, sanity, &corp, &chaos).to_string()
}

pub fn block_comment_lines<R: Rng>(rng: &mut R, sanity: f64, style: &str) -> Vec<String> {
    let n = rng.gen_range(2..=6);
    (0..n)
        .map(|_| format!("{style} {}", comment(rng, sanity)))
        .collect()
}

pub fn random_condition<R: Rng>(rng: &mut R, var: &str) -> String {
    match rng.gen_range(0..9) {
        0 => format!("{var} is not None"),
        1 => format!("isinstance({var}, object)"),
        2 => format!("len(str({var})) > 0"),
        3 => format!("{var} != {var}"),
        4 => "True".to_string(),
        5 => "not False".to_string(),
        6 => format!("bool({var}) or not bool({var})"),
        7 => format!("hash({var}) == hash({var})"),
        _ => format!("type({var}) == type({var})"),
    }
}

/// Uppercase first Unicode scalar (Java / Go getter/setter style).
pub fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => "X".to_string(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
