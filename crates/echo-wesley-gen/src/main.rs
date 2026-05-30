// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! CLI that reads Wesley IR JSON from stdin and emits Rust structs/enums for Echo.

use anyhow::{bail, Result};
use clap::Parser;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{self, Read};
use std::path::PathBuf;

/// Create an identifier safely, falling back to a raw identifier for Rust keywords.
fn safe_ident(name: &str) -> proc_macro2::Ident {
    syn::parse_str::<proc_macro2::Ident>(name)
        .unwrap_or_else(|_| proc_macro2::Ident::new_raw(name, proc_macro2::Span::call_site()))
}

mod ir;
use ir::{OpKind, TypeKind, WesleyIR};

const ECHO_IR_VERSION: &str = "echo-ir/v1";
const DEFAULT_CODEC_ID: &str = "le-binary-v1";
const DEFAULT_REGISTRY_VERSION: u32 = 1;
const RESERVED_CONTROL_OP_ID: u32 = u32::MAX;
const WESLEY_CORE_VERSION: &str = "0.0.4";

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Generates Echo Rust artifacts from Wesley IR"
)]
struct Args {
    /// Read GraphQL SDL directly and lower it with wesley-core.
    #[arg(long)]
    schema: Option<PathBuf>,

    /// Optional output path (defaults to stdout)
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Emit code compatible with no_std environments
    #[arg(long, default_value_t = false)]
    no_std: bool,

    /// Emit minicbor Encode/Decode implementations for all types
    #[arg(long, default_value_t = false)]
    minicbor: bool,

    /// Emit warp-core contract-host helpers for installed mutation handlers
    #[arg(long, default_value_t = false)]
    contract_host: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.no_std && args.contract_host {
        bail!("--contract-host requires std and cannot be combined with --no-std");
    }

    let mut ir = if let Some(schema_path) = &args.schema {
        let schema_sdl = std::fs::read_to_string(schema_path)?;
        echo_ir_from_schema_sdl(&schema_sdl)?
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;

        let ir: WesleyIR = serde_json::from_str(&buffer)?;
        validate_version(&ir)?;
        ir
    };
    // Normalize codec_id to the canonical value BEFORE any artifact hash,
    // observer identity, or footprint certificate preimage is computed.
    // Otherwise an IR that declares e.g. "cbor-canon-v1" would carry that
    // value into the preimage while the emitted CODEC_ID const advertises
    // "le-binary-v1", so the artifact would claim the new wire contract
    // under a hash derived from the old one.
    ir.codec_id = Some(DEFAULT_CODEC_ID.to_string());

    let code = generate_rust(&ir, &args)?;

    if let Some(path) = args.out {
        std::fs::write(path, code)?;
    } else {
        println!("{code}");
    }

    Ok(())
}

fn echo_ir_from_schema_sdl(schema_sdl: &str) -> Result<WesleyIR> {
    let l1_ir = wesley_core::lower_schema_sdl(schema_sdl)?;
    let schema_sha256 = wesley_core::compute_registry_hash(&l1_ir)?;
    let mut operations = wesley_core::list_schema_operations_sdl(schema_sdl)?;
    operations.sort_by_key(operation_sort_key);

    let mut used_op_ids = BTreeMap::new();
    let mut ops = Vec::with_capacity(operations.len());
    for operation in operations {
        let op_id = stable_op_id(&operation.operation_type, &operation.field_name);
        if op_id == 0 {
            bail!(
                "generated operation id collision sentinel for {:?} `{}`; \
                 add explicit operation ids upstream before generating Echo artifacts",
                operation.operation_type,
                operation.field_name
            );
        }
        if op_id == RESERVED_CONTROL_OP_ID {
            bail!(
                "generated operation id for {:?} `{}` uses Echo's reserved control op id; \
                 add explicit operation ids upstream before generating Echo artifacts",
                operation.operation_type,
                operation.field_name
            );
        }
        if let Some((existing_type, existing_name)) = used_op_ids.insert(
            op_id,
            (operation.operation_type, operation.field_name.clone()),
        ) {
            bail!(
                "generated operation id collision: {:?} `{}` and {:?} `{}` both map to {op_id}; \
                 add explicit operation ids upstream before generating Echo artifacts",
                existing_type,
                existing_name,
                operation.operation_type,
                operation.field_name
            );
        }

        ops.push(ir::OpDefinition {
            kind: op_kind_from_wesley(operation.operation_type),
            name: operation.field_name,
            op_id,
            args: operation
                .arguments
                .into_iter()
                .map(|argument| ir::ArgDefinition {
                    name: argument.name,
                    type_name: argument.r#type.base,
                    required: !argument.r#type.nullable,
                    list: argument.r#type.is_list,
                })
                .collect(),
            result_type: operation.result_type.base,
            directives: serde_json::to_value(operation.directives)?,
        });
    }

    Ok(WesleyIR {
        ir_version: Some(ECHO_IR_VERSION.to_string()),
        generated_by: Some(ir::GeneratedBy {
            tool: "wesley-core".to_string(),
            version: Some(WESLEY_CORE_VERSION.to_string()),
        }),
        schema_sha256: Some(schema_sha256),
        types: l1_ir
            .types
            .into_iter()
            .map(type_definition_from_wesley)
            .collect(),
        ops,
        codec_id: Some(DEFAULT_CODEC_ID.to_string()),
        registry_version: Some(DEFAULT_REGISTRY_VERSION),
    })
}

fn operation_sort_key(operation: &wesley_core::SchemaOperation) -> (u8, String) {
    (
        operation_type_rank(operation.operation_type),
        operation.field_name.clone(),
    )
}

fn operation_type_rank(operation_type: wesley_core::OperationType) -> u8 {
    match operation_type {
        wesley_core::OperationType::Query => 0,
        wesley_core::OperationType::Mutation => 1,
        wesley_core::OperationType::Subscription => 2,
    }
}

/// FNV-1 32-bit op id derivation. Despite an earlier "1a" misnomer in this
/// file, the actual byte step multiplies first and then xors (`hash *
/// prime ^ byte`), which is FNV-1, not FNV-1a (`(hash ^ byte) * prime`).
/// The cross-language op-id contract — including the pinned vectors in
/// `stable_op_id_pinned` — is locked to this FNV-1 ordering. Must stay
/// bytewise identical to `wesley_core::stable_op_id` (added in wesley-core
/// ≥0.0.5). The duplication will collapse when echo bumps its wesley-core
/// dependency to 0.0.5+; until then both copies are pinned to the same
/// outputs in unit tests.
fn stable_op_id(operation_type: &wesley_core::OperationType, field_name: &str) -> u32 {
    let mut hash = 2_166_136_261_u32;
    hash = fnv1_step(hash, operation_type_rank(*operation_type));
    for byte in field_name.as_bytes() {
        hash = fnv1_step(hash, *byte);
    }
    hash
}

fn fnv1_step(hash: u32, byte: u8) -> u32 {
    hash.wrapping_mul(16_777_619) ^ u32::from(byte)
}

fn op_kind_from_wesley(operation_type: wesley_core::OperationType) -> OpKind {
    match operation_type {
        wesley_core::OperationType::Query | wesley_core::OperationType::Subscription => {
            OpKind::Query
        }
        wesley_core::OperationType::Mutation => OpKind::Mutation,
    }
}

fn type_definition_from_wesley(type_definition: wesley_core::TypeDefinition) -> ir::TypeDefinition {
    ir::TypeDefinition {
        name: type_definition.name,
        kind: type_kind_from_wesley(type_definition.kind),
        fields: type_definition
            .fields
            .into_iter()
            .map(|field| ir::FieldDefinition {
                name: field.name,
                type_name: field.r#type.base,
                required: !field.r#type.nullable,
                list: field.r#type.is_list,
            })
            .collect(),
        values: type_definition.enum_values,
    }
}

fn type_kind_from_wesley(type_kind: wesley_core::TypeKind) -> TypeKind {
    match type_kind {
        wesley_core::TypeKind::Object => TypeKind::Object,
        wesley_core::TypeKind::Interface => TypeKind::Interface,
        wesley_core::TypeKind::Union => TypeKind::Union,
        wesley_core::TypeKind::Enum => TypeKind::Enum,
        wesley_core::TypeKind::Scalar => TypeKind::Scalar,
        wesley_core::TypeKind::InputObject => TypeKind::InputObject,
    }
}

fn generate_rust(ir: &WesleyIR, args: &Args) -> Result<String> {
    validate_operation_ids(ir)?;
    validate_generated_item_names(ir)?;

    let mut tokens = quote! {
        // Generated by echo-wesley-gen. Do not edit.
    };

    if args.no_std {
        tokens.extend(quote! {
            extern crate alloc;
            use alloc::string::String;
            use alloc::vec::Vec;
        });
    }

    // Bring the codec traits into scope at the top of the generated module
    // so Encode/Decode impl bodies can call .encode(w) / Type::decode(r) on
    // nested user types via method/associated-fn syntax without needing a
    // fully-qualified path at every call site.
    tokens.extend(quote! {
        use serde::{Serialize, Deserialize};
        use echo_wasm_abi::codec::{Decode as _, Encode as _};
    });

    if args.minicbor {
        tokens.extend(quote! {
            use minicbor::{Encode, Decode};
        });
    }

    // Metadata constants
    let schema_sha = ir.schema_sha256.as_deref().unwrap_or("");
    let codec_id = DEFAULT_CODEC_ID;
    let registry_version = ir.registry_version.unwrap_or(1);
    let generated_rust_artifact_hash = generated_rust_artifact_hash(ir, args)?;
    let wesley_generator_version = format!("echo-wesley-gen/{}", env!("CARGO_PKG_VERSION"));

    tokens.extend(quote! {
        pub const SCHEMA_SHA256: &str = #schema_sha;
        pub const ECHO_CONTRACT_ABI_VERSION: u32 = 1;
        pub const CODEC_ID: &str = #codec_id;
        pub const REGISTRY_VERSION: u32 = #registry_version;
        pub const WESLEY_GENERATOR_VERSION: &str = #wesley_generator_version;
        pub const CONTRACT_HOST_HELPER_API_VERSION: u32 = 1;
        pub const GENERATED_RUST_ARTIFACT_HASH: &str = #generated_rust_artifact_hash;
    });

    for type_def in &ir.types {
        let name = safe_ident(&type_def.name);

        let mut derives = quote! { Debug, Clone, PartialEq, Serialize, Deserialize };
        if args.minicbor {
            derives.extend(quote! { , Encode, Decode });
        }

        match type_def.kind {
            TypeKind::Enum => {
                let variants: Vec<_> = type_def.values.iter().map(|v| safe_ident(v)).collect();
                tokens.extend(quote! {
                    #[derive(#derives, Copy, Eq)]
                    #[cbor(index_only)]
                    pub enum #name {
                        #(#variants),*
                    }
                });
                // Emit LE binary Encode/Decode for Enum types.
                let variant_arms = type_def.values.iter().enumerate().map(|(i, _v)| {
                    let variant = safe_ident(&type_def.values[i]);
                    // Safety: realistic enums have far fewer than u32::MAX variants.
                    #[allow(clippy::cast_possible_truncation)]
                    let discriminant = i as u32;
                    quote! { #discriminant => Ok(Self::#variant) }
                });
                // Safety: realistic enums have far fewer than u32::MAX variants.
                #[allow(clippy::cast_possible_truncation)]
                let num_variants = type_def.values.len() as u32;
                tokens.extend(quote! {
                    impl echo_wasm_abi::codec::Encode for #name {
                        fn encode(&self, w: &mut echo_wasm_abi::codec::Writer) -> Result<(), echo_wasm_abi::codec::CodecError> {
                            w.write_u32_le(*self as u32);
                            Ok(())
                        }
                    }
                    impl echo_wasm_abi::codec::Decode for #name {
                        fn decode(r: &mut echo_wasm_abi::codec::Reader<'_>) -> Result<Self, echo_wasm_abi::codec::CodecError> {
                            let discriminant = r.read_u32_le()?;
                            match discriminant {
                                #(#variant_arms,)*
                                _ => Err(echo_wasm_abi::codec::CodecError::InvalidEnum),
                            }
                        }
                    }
                });
                // Reject discriminant values >= num_variants to ensure exhaustive match.
                let _ = num_variants;
            }
            TypeKind::InputObject => {
                let fields = type_def.fields.iter().enumerate().map(|(i, f)| {
                    let field_name = safe_ident(&f.name);
                    let base_ty = map_type(&f.type_name, args);
                    let list_ty: TokenStream = if f.list {
                        quote! { Vec<#base_ty> }
                    } else {
                        quote! { #base_ty }
                    };

                    let field_tokens = if f.required {
                        quote! { pub #field_name: #list_ty }
                    } else {
                        quote! { pub #field_name: Option<#list_ty> }
                    };

                    if args.minicbor {
                        let idx = i as u64;
                        quote! {
                            #[n(#idx)]
                            #field_tokens
                        }
                    } else {
                        field_tokens
                    }
                });

                tokens.extend(quote! {
                    #[derive(#derives)]
                    pub struct #name {
                        #(#fields),*
                    }
                });

                // Emit LE binary Encode/Decode for InputObject types.
                let encode_stmts: Vec<TokenStream> = type_def
                    .fields
                    .iter()
                    .map(|f| encode_field_stmt(f, args))
                    .collect();
                let decode_fields: Vec<TokenStream> = type_def
                    .fields
                    .iter()
                    .map(|f| decode_field_expr(f, args))
                    .collect();
                tokens.extend(quote! {
                    impl echo_wasm_abi::codec::Encode for #name {
                        fn encode(&self, w: &mut echo_wasm_abi::codec::Writer) -> Result<(), echo_wasm_abi::codec::CodecError> {
                            #(#encode_stmts)*
                            Ok(())
                        }
                    }
                    impl echo_wasm_abi::codec::Decode for #name {
                        fn decode(r: &mut echo_wasm_abi::codec::Reader<'_>) -> Result<Self, echo_wasm_abi::codec::CodecError> {
                            Ok(Self {
                                #(#decode_fields),*
                            })
                        }
                    }
                });
            }
            TypeKind::Object => {
                let fields = type_def.fields.iter().enumerate().map(|(i, f)| {
                    let field_name = safe_ident(&f.name);
                    let base_ty = map_type(&f.type_name, args);
                    let list_ty: TokenStream = if f.list {
                        quote! { Vec<#base_ty> }
                    } else {
                        quote! { #base_ty }
                    };

                    let field_tokens = if f.required {
                        quote! { pub #field_name: #list_ty }
                    } else {
                        quote! { pub #field_name: Option<#list_ty> }
                    };

                    if args.minicbor {
                        let idx = i as u64;
                        quote! {
                            #[n(#idx)]
                            #field_tokens
                        }
                    } else {
                        field_tokens
                    }
                });

                tokens.extend(quote! {
                    #[derive(#derives)]
                    pub struct #name {
                        #(#fields),*
                    }
                });
            }
            _ => {} // Ignore scalars/interfaces for now
        }
    }

    if !ir.ops.is_empty() {
        let mut ops_sorted: Vec<_> = ir.ops.iter().collect();
        ops_sorted.sort_unstable_by_key(|op| op.op_id);
        let footprint_certificates = ops_sorted
            .iter()
            .map(|op| {
                let certificate = op_footprint_certificate(ir, op, &generated_rust_artifact_hash)?;
                Ok((op.op_id, certificate))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let has_footprint_certificates = footprint_certificates.values().any(Option::is_some);

        if has_footprint_certificates {
            tokens.extend(quote! {
                // Registry provider types (Echo runtime loads an app-supplied implementation).
                use echo_registry_api::{ArgDef, EnumDef, FootprintCertificate, ObjectDef, OpDef, OpKind, RegistryInfo, RegistryProvider};
            });
        } else {
            tokens.extend(quote! {
                // Registry provider types (Echo runtime loads an app-supplied implementation).
                use echo_registry_api::{ArgDef, EnumDef, ObjectDef, OpDef, OpKind, RegistryInfo, RegistryProvider};
            });
        }

        let mut enum_defs: Vec<_> = ir
            .types
            .iter()
            .filter(|t| t.kind == TypeKind::Enum)
            .collect();
        enum_defs.sort_unstable_by(|a, b| a.name.cmp(&b.name));

        for en in &enum_defs {
            let values_ident = format_ident!("ENUM_{}_VALUES", en.name.to_ascii_uppercase());
            let values = en.values.iter();
            tokens.extend(quote! {
                pub const #values_ident: &[&str] = &[
                    #(#values),*
                ];
            });
        }

        let enum_entries = enum_defs.iter().map(|en| {
            let name = &en.name;
            let values_ident = format_ident!("ENUM_{}_VALUES", en.name.to_ascii_uppercase());
            quote! { EnumDef { name: #name, values: #values_ident } }
        });

        tokens.extend(quote! {
            pub const ENUMS: &[EnumDef] = &[
                #(#enum_entries),*
            ];
        });

        let mut obj_defs: Vec<_> = ir
            .types
            .iter()
            .filter(|t| t.kind == TypeKind::Object)
            .collect();
        obj_defs.sort_unstable_by(|a, b| a.name.cmp(&b.name));

        for obj in &obj_defs {
            let fields_ident = format_ident!("OBJ_{}_FIELDS", obj.name.to_ascii_uppercase());
            let fields = obj.fields.iter().map(|f| {
                let name = &f.name;
                let ty = &f.type_name;
                let required = f.required;
                let list = f.list;
                quote! { ArgDef { name: #name, ty: #ty, required: #required, list: #list } }
            });
            tokens.extend(quote! {
                pub const #fields_ident: &[ArgDef] = &[
                    #(#fields),*
                ];
            });
        }

        let obj_entries = obj_defs.iter().map(|obj| {
            let name = &obj.name;
            let fields_ident = format_ident!("OBJ_{}_FIELDS", obj.name.to_ascii_uppercase());
            quote! { ObjectDef { name: #name, fields: #fields_ident } }
        });

        tokens.extend(quote! {
            pub const OBJECTS: &[ObjectDef] = &[
                #(#obj_entries),*
            ];
        });

        // Op ID constants + arg descriptors + footprint certificates
        // (sorted by op_id for deterministic iteration).
        for op in &ops_sorted {
            let const_name = op_const_ident(&op.name, op.op_id);
            let args_name = format_ident!("{}_ARGS", const_name);
            let op_id = op.op_id;
            let args = op.args.iter().map(|a| {
                let name = &a.name;
                let ty = &a.type_name;
                let required = a.required;
                let list = a.list;
                quote! { ArgDef { name: #name, ty: #ty, required: #required, list: #list } }
            });
            tokens.extend(quote! {
                pub const #const_name: u32 = #op_id;
                pub const #args_name: &[ArgDef] = &[
                    #(#args),*
                ];
            });

            if let Some(certificate) = footprint_certificates
                .get(&op.op_id)
                .and_then(|value| value.as_ref())
            {
                let reads_name = format_ident!("{}_FOOTPRINT_READS", const_name);
                let writes_name = format_ident!("{}_FOOTPRINT_WRITES", const_name);
                let artifact_hash_name = format_ident!("{}_FOOTPRINT_ARTIFACT_HASH", const_name);
                let certificate_hash_name =
                    format_ident!("{}_FOOTPRINT_CERTIFICATE_HASH", const_name);
                let certificate_name = format_ident!("{}_FOOTPRINT_CERTIFICATE", const_name);
                let op_name = &op.name;
                let reads = certificate.reads.iter();
                let writes = certificate.writes.iter();
                let artifact_hash = certificate.artifact_hash_hex.as_str();
                let certificate_hash = certificate.certificate_hash_hex.as_str();
                tokens.extend(quote! {
                    pub const #reads_name: &[&str] = &[
                        #(#reads),*
                    ];
                    pub const #writes_name: &[&str] = &[
                        #(#writes),*
                    ];
                    pub const #artifact_hash_name: &str = #artifact_hash;
                    pub const #certificate_hash_name: &str = #certificate_hash;
                    pub const #certificate_name: FootprintCertificate = FootprintCertificate {
                        op_id: #const_name,
                        op_name: #op_name,
                        schema_sha256_hex: SCHEMA_SHA256,
                        artifact_hash_hex: #artifact_hash_name,
                        certificate_hash_hex: #certificate_hash_name,
                        reads: #reads_name,
                        writes: #writes_name,
                    };
                });
            }
        }

        let mut helper_prelude = TokenStream::new();
        let mut helper_tokens = TokenStream::new();
        let mut helper_exports = Vec::new();

        if args.no_std {
            helper_prelude.extend(quote! {
                use alloc::string::String;
                use alloc::vec::Vec;
            });
        }

        // Vars Encode/Decode impls inside __echo_wesley_generated call
        // .encode(w) / Type::decode(r) on user-defined types that live in
        // the parent module; without these imports the trait methods are
        // unresolvable from inside the private module.
        helper_prelude.extend(quote! {
            use echo_wasm_abi::codec::{Decode as _, Encode as _};
        });

        let has_query_ops = ir.ops.iter().any(|op| op.kind == OpKind::Query);
        let has_mutation_ops = ir.ops.iter().any(|op| op.kind == OpKind::Mutation);

        if has_query_ops {
            helper_prelude.extend(quote! {
                use echo_wasm_abi::kernel_port::{
                    AttachmentDescentPolicy, EchoCoordinate, ObserveOpticRequest, OpticAperture,
                    OpticApertureShape, OpticCapabilityId, OpticFocus, OpticId, OpticReadBudget,
                    ObservationAt, ObservationCoordinate, ObservationFrame, ObservationProjection,
                    ObservationRequest, ProjectionVersion, ReducerVersion, WorldlineId,
                };
            });
        }

        if has_mutation_ops {
            if has_query_ops {
                helper_prelude.extend(quote! {
                    use echo_wasm_abi::kernel_port::{
                        AdmissionLawId, DispatchOpticIntentRequest, IntentFamilyId,
                        OpticCapability, OpticCause, OpticIntentPayload,
                    };
                });
            } else {
                helper_prelude.extend(quote! {
                    use echo_wasm_abi::kernel_port::{
                        AdmissionLawId, DispatchOpticIntentRequest, EchoCoordinate,
                        IntentFamilyId, OpticCapability, OpticCause, OpticFocus, OpticId,
                        OpticIntentPayload,
                    };
                });
            }
            helper_prelude.extend(quote! {
                use echo_wasm_abi::pack_intent_v1;

                /// Error produced while building a generated EINT intent.
                #[derive(Debug)]
                pub enum GeneratedIntentError {
                    /// Operation vars could not be encoded.
                    EncodeVars(echo_wasm_abi::codec::CodecError),
                    /// Encoded vars could not be packed into an EINT envelope.
                    PackEnvelope(echo_wasm_abi::EnvelopeError),
                }
            });
        }

        if args.contract_host && has_mutation_ops {
            helper_prelude.extend(quote! {
                use warp_core::{
                    ConflictPolicy, Footprint, GraphView, NodeId, PatternGraph, RewriteRule,
                    TickDelta,
                };
            });
        }

        if has_query_ops {
            helper_tokens.extend(quote! {
                fn generated_vars_digest(vars_bytes: &[u8]) -> Vec<u8> {
                    echo_wasm_abi::query_vars_digest_v1(vars_bytes)
                }
            });
        }

        for op in &ops_sorted {
            let const_name = op_const_ident(&op.name, op.op_id);
            let helper_name_string = to_snake_case(&op.name);
            let helper_name = format_ident!("{}", helper_name_string);
            let vars_name = format_ident!("{}Vars", to_pascal_case(&op.name));
            let vars_fields: Vec<TokenStream> = op
                .args
                .iter()
                .map(|a| {
                    let field_name = safe_ident(&a.name);
                    let base_ty = map_helper_type(&a.type_name, args);
                    let list_ty: TokenStream = if a.list {
                        quote! { Vec<#base_ty> }
                    } else {
                        quote! { #base_ty }
                    };

                    if a.required {
                        quote! { pub #field_name: #list_ty }
                    } else {
                        quote! { pub #field_name: Option<#list_ty> }
                    }
                })
                .collect();
            let encode_fn_name = format_ident!("encode_{}_vars", helper_name);
            helper_exports.push(encode_fn_name.clone());

            // Encode/Decode impls for the Vars struct (use super:: for user-defined types).
            let vars_encode_stmts: Vec<TokenStream> =
                op.args.iter().map(|a| encode_arg_stmt(a, args)).collect();
            let vars_decode_fields: Vec<TokenStream> =
                op.args.iter().map(|a| decode_arg_expr(a, args)).collect();

            helper_tokens.extend(quote! {
                /// LE binary vars payload for this generated operation.
                #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
                pub struct #vars_name {
                    #(#vars_fields),*
                }

                impl echo_wasm_abi::codec::Encode for #vars_name {
                    fn encode(&self, w: &mut echo_wasm_abi::codec::Writer) -> Result<(), echo_wasm_abi::codec::CodecError> {
                        #(#vars_encode_stmts)*
                        Ok(())
                    }
                }

                impl echo_wasm_abi::codec::Decode for #vars_name {
                    fn decode(r: &mut echo_wasm_abi::codec::Reader<'_>) -> Result<Self, echo_wasm_abi::codec::CodecError> {
                        Ok(Self {
                            #(#vars_decode_fields),*
                        })
                    }
                }

                /// Encode this operation's vars using the LE binary codec.
                pub fn #encode_fn_name(vars: &#vars_name) -> Result<Vec<u8>, echo_wasm_abi::codec::CodecError> {
                    echo_wasm_abi::codec::encode_to_vec(vars)
                }
            });
            match op.kind {
                OpKind::Mutation => {
                    let fn_name = format_ident!("pack_{}_intent", helper_name);
                    let raw_fn_name = format_ident!("pack_{}_intent_raw_vars", helper_name);
                    let optic_helper_name =
                        format_ident!("{}", optic_mutation_helper_stem(&op.name));
                    let optic_fn_name =
                        format_ident!("{}_dispatch_optic_intent_request", optic_helper_name);
                    let optic_raw_fn_name = format_ident!(
                        "{}_dispatch_optic_intent_request_raw_vars",
                        optic_helper_name
                    );
                    helper_exports.push(fn_name.clone());
                    helper_exports.push(raw_fn_name.clone());
                    helper_exports.push(optic_fn_name.clone());
                    helper_exports.push(optic_raw_fn_name.clone());
                    let contract_rule_name_const =
                        format_ident!("{}_CONTRACT_RULE_NAME", const_name);
                    let contract_rule_id_label_const =
                        format_ident!("{}_CONTRACT_RULE_ID_LABEL", const_name);
                    let contract_match_fn_name = format_ident!("{}_contract_matches", helper_name);
                    let contract_vars_fn_name = format_ident!("{}_contract_vars", helper_name);
                    let contract_footprint_fn_name =
                        format_ident!("{}_contract_runtime_ingress_footprint", helper_name);
                    let contract_rule_fn_name = format_ident!("{}_contract_rule", helper_name);
                    let contract_rule_name =
                        format!("cmd/contract/{}/{}/{}", schema_sha, op.op_id, op.name);
                    let contract_rule_id_label = format!("rule:{contract_rule_name}");
                    helper_tokens.extend(quote! {
                        /// Encode this mutation's vars and pack them into an EINT v1 intent.
                        pub fn #fn_name(vars: &#vars_name) -> Result<Vec<u8>, GeneratedIntentError> {
                            let vars_bytes = #encode_fn_name(vars).map_err(GeneratedIntentError::EncodeVars)?;
                            pack_intent_v1(super::#const_name, &vars_bytes).map_err(GeneratedIntentError::PackEnvelope)
                        }

                        /// Pack already-canonical vars bytes for this generated mutation into EINT v1.
                        pub fn #raw_fn_name(vars: &[u8]) -> Result<Vec<u8>, echo_wasm_abi::EnvelopeError> {
                            pack_intent_v1(super::#const_name, vars)
                        }

                        /// Build an optic intent-dispatch request for this mutation.
                        #[allow(clippy::too_many_arguments)]
                        pub fn #optic_fn_name(
                            optic_id: OpticId,
                            base_coordinate: EchoCoordinate,
                            intent_family: IntentFamilyId,
                            focus: OpticFocus,
                            cause: OpticCause,
                            capability: OpticCapability,
                            admission_law: AdmissionLawId,
                            vars: &#vars_name,
                        ) -> Result<DispatchOpticIntentRequest, GeneratedIntentError> {
                            let vars_bytes = #encode_fn_name(vars).map_err(GeneratedIntentError::EncodeVars)?;
                            #optic_raw_fn_name(
                                optic_id,
                                base_coordinate,
                                intent_family,
                                focus,
                                cause,
                                capability,
                                admission_law,
                                &vars_bytes,
                            )
                        }

                        /// Build an optic intent-dispatch request from already-canonical vars bytes.
                        #[allow(clippy::too_many_arguments)]
                        pub fn #optic_raw_fn_name(
                            optic_id: OpticId,
                            base_coordinate: EchoCoordinate,
                            intent_family: IntentFamilyId,
                            focus: OpticFocus,
                            cause: OpticCause,
                            capability: OpticCapability,
                            admission_law: AdmissionLawId,
                            vars: &[u8],
                        ) -> Result<DispatchOpticIntentRequest, GeneratedIntentError> {
                            let bytes = pack_intent_v1(super::#const_name, vars)
                                .map_err(GeneratedIntentError::PackEnvelope)?;
                            Ok(DispatchOpticIntentRequest {
                                optic_id,
                                base_coordinate,
                                intent_family,
                                focus,
                                cause,
                                capability,
                                admission_law,
                                payload: OpticIntentPayload::EintV1 { bytes },
                            })
                        }
                    });
                    if args.contract_host {
                        helper_exports.push(contract_rule_name_const.clone());
                        helper_exports.push(contract_rule_id_label_const.clone());
                        helper_exports.push(contract_match_fn_name.clone());
                        helper_exports.push(contract_vars_fn_name.clone());
                        helper_exports.push(contract_footprint_fn_name.clone());
                        helper_exports.push(contract_rule_fn_name.clone());
                        helper_tokens.extend(quote! {
                            /// Stable command-rule name for this generated contract mutation.
                            pub const #contract_rule_name_const: &str = #contract_rule_name;

                            /// Stable rule-id label for this generated contract mutation.
                            pub const #contract_rule_id_label_const: &str = #contract_rule_id_label;

                            /// Return true when a scheduler-materialized runtime ingress event
                            /// carries this mutation's EINT operation id.
                            pub fn #contract_match_fn_name(view: GraphView<'_>, scope: &NodeId) -> bool {
                                warp_core::matches_eint_op(view, scope, super::#const_name)
                            }

                            /// Decode this mutation's generated vars from a scheduler-materialized
                            /// EINT runtime ingress event.
                            pub fn #contract_vars_fn_name(view: GraphView<'_>, scope: &NodeId) -> Option<#vars_name> {
                                let vars = warp_core::eint_vars_for_op(view, scope, super::#const_name)?;
                                echo_wasm_abi::codec::decode_from_bytes(vars).ok()
                            }

                            /// Base footprint for reading this mutation's runtime ingress event.
                            ///
                            /// Installed executors must extend this with their handler-specific
                            /// graph, edge, attachment, and port writes.
                            pub fn #contract_footprint_fn_name(view: GraphView<'_>, scope: &NodeId) -> Footprint {
                                warp_core::runtime_ingress_eint_read_footprint(view, scope)
                            }

                            /// Build a `warp-core` command rule for this generated contract
                            /// mutation using a host-supplied executor and footprint function.
                            pub fn #contract_rule_fn_name(
                                executor: for<'a> fn(GraphView<'a>, &NodeId, &mut TickDelta),
                                compute_footprint: for<'a> fn(GraphView<'a>, &NodeId) -> Footprint,
                            ) -> RewriteRule {
                                RewriteRule {
                                    id: warp_core::make_type_id(#contract_rule_id_label_const).0,
                                    name: #contract_rule_name_const,
                                    left: PatternGraph { nodes: Vec::new() },
                                    matcher: #contract_match_fn_name,
                                    executor,
                                    compute_footprint,
                                    factor_mask: 0,
                                    conflict_policy: ConflictPolicy::Abort,
                                    join_fn: None,
                                }
                            }
                        });
                    }
                }
                OpKind::Query => {
                    let fn_name = format_ident!("{}_observation_request", helper_name);
                    let raw_fn_name = format_ident!("{}_observation_request_raw_vars", helper_name);
                    let optic_fn_name = format_ident!("{}_observe_optic_request", helper_name);
                    let optic_raw_fn_name =
                        format_ident!("{}_observe_optic_request_raw_vars", helper_name);
                    let observer_plan_label_const =
                        format_ident!("{}_OBSERVER_PLAN_ID_LABEL", const_name);
                    let observer_artifact_hash_const =
                        format_ident!("{}_OBSERVER_ARTIFACT_HASH", const_name);
                    let observer_plan_fn_name = format_ident!("{}_observer_plan", helper_name);
                    let observer_vars_fn_name = format_ident!("{}_observer_vars", helper_name);
                    let query_observer_fn_name = format_ident!("{}_query_observer", helper_name);
                    helper_exports.push(fn_name.clone());
                    helper_exports.push(raw_fn_name.clone());
                    helper_exports.push(optic_fn_name.clone());
                    helper_exports.push(optic_raw_fn_name.clone());
                    helper_tokens.extend(quote! {
                        /// Encode this query's vars and build a frontier query-view observation request.
                        pub fn #fn_name(worldline_id: WorldlineId, vars: &#vars_name) -> Result<ObservationRequest, echo_wasm_abi::codec::CodecError> {
                            let vars_bytes = #encode_fn_name(vars)?;
                            Ok(#raw_fn_name(worldline_id, &vars_bytes))
                        }

                        /// Build a frontier query-view request from already-canonical vars bytes.
                        pub fn #raw_fn_name(worldline_id: WorldlineId, vars: &[u8]) -> ObservationRequest {
                            ObservationRequest::builtin_one_shot(
                                ObservationCoordinate {
                                    worldline_id,
                                    at: ObservationAt::Frontier,
                                },
                                ObservationFrame::QueryView,
                                ObservationProjection::Query {
                                    query_id: super::#const_name,
                                    vars_bytes: Vec::from(vars),
                                },
                            )
                            .expect("generated query observation request uses a valid frame/projection pair")
                        }

                        /// Encode this query's vars and build a bounded optic read request.
                        #[allow(clippy::too_many_arguments)]
                        pub fn #optic_fn_name(
                            optic_id: OpticId,
                            focus: OpticFocus,
                            coordinate: EchoCoordinate,
                            capability: OpticCapabilityId,
                            projection_version: ProjectionVersion,
                            reducer_version: Option<ReducerVersion>,
                            budget: OpticReadBudget,
                            vars: &#vars_name,
                        ) -> Result<ObserveOpticRequest, echo_wasm_abi::codec::CodecError> {
                            let vars_bytes = #encode_fn_name(vars)?;
                            Ok(#optic_raw_fn_name(
                                optic_id,
                                focus,
                                coordinate,
                                capability,
                                projection_version,
                                reducer_version,
                                budget,
                                &vars_bytes,
                            ))
                        }

                        /// Build a bounded optic read request from already-canonical vars bytes.
                        #[allow(clippy::too_many_arguments)]
                        pub fn #optic_raw_fn_name(
                            optic_id: OpticId,
                            focus: OpticFocus,
                            coordinate: EchoCoordinate,
                            capability: OpticCapabilityId,
                            projection_version: ProjectionVersion,
                            reducer_version: Option<ReducerVersion>,
                            budget: OpticReadBudget,
                            vars: &[u8],
                        ) -> ObserveOpticRequest {
                            ObserveOpticRequest {
                                optic_id,
                                focus,
                                coordinate,
                                aperture: OpticAperture {
                                    shape: OpticApertureShape::QueryBytes {
                                        query_id: super::#const_name,
                                        vars_digest: generated_vars_digest(vars),
                                    },
                                    budget,
                                    attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
                                },
                                projection_version,
                                reducer_version,
                                capability,
                            }
                        }
                    });
                    if args.contract_host {
                        helper_exports.push(observer_plan_label_const.clone());
                        helper_exports.push(observer_artifact_hash_const.clone());
                        helper_exports.push(observer_plan_fn_name.clone());
                        helper_exports.push(observer_vars_fn_name.clone());
                        helper_exports.push(query_observer_fn_name.clone());
                        let observer_plan_label =
                            format!("observer:query/{schema_sha}/{}/{}", op.op_id, op.name);
                        let observer_identity =
                            observer_identity_hashes(ir, op, &generated_rust_artifact_hash)?;
                        let observer_plan_id = byte_array_literal(observer_identity.plan_id);
                        let observer_artifact_hash =
                            byte_array_literal(observer_identity.artifact_hash);
                        let observer_schema_hash =
                            byte_array_literal(observer_identity.schema_hash);
                        let observer_state_schema_hash =
                            byte_array_literal(observer_identity.state_schema_hash);
                        let observer_update_law_hash =
                            byte_array_literal(observer_identity.update_law_hash);
                        let observer_emission_law_hash =
                            byte_array_literal(observer_identity.emission_law_hash);
                        let observer_artifact_hash_hex = hash_hex(observer_identity.artifact_hash);
                        helper_tokens.extend(quote! {
                            /// Stable authored observer plan-id label for this generated query.
                            pub const #observer_plan_label_const: &str = #observer_plan_label;

                            /// Stable artifact hash for this generated query observer helper.
                            pub const #observer_artifact_hash_const: &str = #observer_artifact_hash_hex;

                            /// Build the authored observer plan stamped onto readings emitted
                            /// by this generated query observer.
                            pub fn #observer_plan_fn_name() -> warp_core::AuthoredObserverPlan {
                                warp_core::AuthoredObserverPlan {
                                    plan_id: warp_core::ObserverPlanId::from_bytes(#observer_plan_id),
                                    artifact_hash: #observer_artifact_hash,
                                    schema_hash: #observer_schema_hash,
                                    state_schema_hash: #observer_state_schema_hash,
                                    update_law_hash: #observer_update_law_hash,
                                    emission_law_hash: #observer_emission_law_hash,
                                }
                            }

                            /// Decode this query's generated vars from read-only observer context.
                            pub fn #observer_vars_fn_name(
                                context: &warp_core::ContractQueryObserverContext<'_>,
                            ) -> Result<#vars_name, echo_wasm_abi::codec::CodecError> {
                                echo_wasm_abi::codec::decode_from_bytes(context.vars_bytes)
                            }

                            /// Build a read-only `warp-core` query observer for this generated query.
                            pub fn #query_observer_fn_name<F>(observe: F) -> warp_core::ContractQueryObserver
                            where
                                F: for<'a> Fn(
                                        &warp_core::ContractQueryObserverContext<'a>,
                                        #vars_name,
                                    ) -> Result<
                                        warp_core::ContractQueryObserverResult,
                                        warp_core::ContractQueryObserverError,
                                    >
                                    + Send
                                    + Sync
                                    + 'static,
                            {
                                warp_core::ContractQueryObserver::new(
                                    super::#const_name,
                                    #observer_plan_fn_name(),
                                    move |context| {
                                        let vars = #observer_vars_fn_name(&context).map_err(|error| {
                                            warp_core::ContractQueryObserverError::invalid_vars(
                                                context.query_id,
                                                error.to_string(),
                                            )
                                        })?;
                                        observe(&context, vars)
                                    },
                                )
                            }
                        });
                    }
                }
            }
        }

        tokens.extend(quote! {
            /// Generated operation helper namespace.
            ///
            /// Helper-only types live here so user-controlled Wesley types can
            /// use names such as `IncrementVars` or `GeneratedIntentError`
            /// without colliding with generated plumbing.
            pub mod __echo_wesley_generated {
                #helper_prelude
                #helper_tokens
            }

            pub use __echo_wesley_generated::{
                #(#helper_exports),*
            };
        });

        // OPS table (sorted by op_id).
        let ops_entries = ops_sorted
            .iter()
            .map(|op| {
                let kind = match op.kind {
                    OpKind::Query => quote! { OpKind::Query },
                    OpKind::Mutation => quote! { OpKind::Mutation },
                };
                let name = &op.name;
                let op_id = op.op_id;
                let args_name = format_ident!("{}_ARGS", op_const_ident(&op.name, op.op_id));
                let result_ty = &op.result_type;
                let directives_json = op_directives_json(op)?;
                let footprint_certificate = if footprint_certificates
                    .get(&op.op_id)
                    .and_then(|value| value.as_ref())
                    .is_some()
                {
                    let const_name = op_const_ident(&op.name, op.op_id);
                    let certificate_name = format_ident!("{}_FOOTPRINT_CERTIFICATE", const_name);
                    quote! { Some(&#certificate_name) }
                } else {
                    quote! { None }
                };
                Ok(quote! {
                    OpDef {
                        kind: #kind,
                        name: #name,
                        op_id: #op_id,
                        args: #args_name,
                        result_ty: #result_ty,
                        directives_json: #directives_json,
                        footprint_certificate: #footprint_certificate,
                    }
                })
            })
            .collect::<Result<Vec<_>>>()?;

        tokens.extend(quote! {
            pub const OPS: &[OpDef] = &[
                #(#ops_entries),*
            ];

            /// Lookup an op by ID.
            pub fn op_by_id(op_id: u32) -> Option<&'static OpDef> {
                OPS.iter().find(|op| op.op_id == op_id)
            }

            /// Lookup an op by kind + name (useful for dev tooling, not for runtime intent routing).
            pub fn op_by_name(kind: OpKind, name: &str) -> Option<&'static OpDef> {
                OPS.iter().find(|op| op.kind == kind && op.name == name)
            }

            /// Application-supplied registry provider implementation (generated from Wesley IR).
            pub struct GeneratedRegistry;

            impl RegistryProvider for GeneratedRegistry {
                fn info(&self) -> RegistryInfo {
                    RegistryInfo {
                        echo_abi_version: ECHO_CONTRACT_ABI_VERSION,
                        codec_id: CODEC_ID,
                        registry_version: REGISTRY_VERSION,
                        schema_sha256_hex: SCHEMA_SHA256,
                        wesley_generator_version: WESLEY_GENERATOR_VERSION,
                        helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
                    }
                }

                fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef> {
                    op_by_id(op_id)
                }

                fn all_ops(&self) -> &'static [OpDef] {
                    OPS
                }

                fn all_enums(&self) -> &'static [EnumDef] {
                    ENUMS
                }

                fn all_objects(&self) -> &'static [ObjectDef] {
                    OBJECTS
                }
            }

            pub static REGISTRY: GeneratedRegistry = GeneratedRegistry;
        });
    }

    let syntax_tree = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&syntax_tree))
}

fn validate_operation_ids(ir: &WesleyIR) -> Result<()> {
    for op in &ir.ops {
        if op.op_id == RESERVED_CONTROL_OP_ID {
            bail!(
                "operation `{}` uses Echo's reserved control op id {RESERVED_CONTROL_OP_ID}; \
                 application contracts must not generate scheduler control intents",
                op.name
            );
        }
    }

    Ok(())
}

fn op_const_ident(name: &str, op_id: u32) -> proc_macro2::Ident {
    format_ident!("{}", op_const_name(name, op_id))
}

fn op_directives_json(op: &ir::OpDefinition) -> Result<String> {
    serde_json::to_string(&op.directives).map_err(Into::into)
}

fn generated_rust_artifact_hash(ir: &WesleyIR, args: &Args) -> Result<String> {
    let mut type_defs = ir.types.iter().collect::<Vec<_>>();
    type_defs.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    let mut op_defs = ir.ops.iter().collect::<Vec<_>>();
    op_defs.sort_unstable_by_key(|op| op.op_id);

    let type_catalog_json = serde_json::to_string(&type_defs)?;
    let op_catalog_json = serde_json::to_string(&op_defs)?;
    let schema_sha = ir.schema_sha256.as_deref().unwrap_or("");
    let codec_id = ir.codec_id.as_deref().unwrap_or(DEFAULT_CODEC_ID);
    let registry_version = ir.registry_version.unwrap_or(DEFAULT_REGISTRY_VERSION);
    let ir_version = ir.ir_version.as_deref().unwrap_or("");
    let generated_by_json = serde_json::to_string(&ir.generated_by)?;

    let preimage = format!(
        concat!(
            "echo-wesley-rust-artifact/v1\n",
            "generator=echo-wesley-gen\n",
            "generator_version={generator_version}\n",
            "ir_version={ir_version}\n",
            "schema_sha256={schema_sha}\n",
            "codec_id={codec_id}\n",
            "registry_version={registry_version}\n",
            "no_std={no_std}\n",
            "minicbor={minicbor}\n",
            "generated_by={generated_by_json}\n",
            "types={type_catalog_json}\n",
            "ops={op_catalog_json}\n",
        ),
        generator_version = env!("CARGO_PKG_VERSION"),
        ir_version = ir_version,
        schema_sha = schema_sha,
        codec_id = codec_id,
        registry_version = registry_version,
        no_std = args.no_std,
        minicbor = args.minicbor,
        generated_by_json = generated_by_json,
        type_catalog_json = type_catalog_json,
        op_catalog_json = op_catalog_json,
    );

    Ok(blake3_hex(preimage.as_bytes()))
}

#[derive(Debug, Clone)]
struct GeneratedFootprintCertificate {
    reads: Vec<String>,
    writes: Vec<String>,
    artifact_hash_hex: String,
    certificate_hash_hex: String,
}

#[derive(Debug, Clone, Copy)]
struct GeneratedObserverIdentity {
    plan_id: [u8; 32],
    artifact_hash: [u8; 32],
    schema_hash: [u8; 32],
    state_schema_hash: [u8; 32],
    update_law_hash: [u8; 32],
    emission_law_hash: [u8; 32],
}

fn observer_identity_hashes(
    ir: &WesleyIR,
    op: &ir::OpDefinition,
    generated_rust_artifact_hash: &str,
) -> Result<GeneratedObserverIdentity> {
    let args_json = serde_json::to_string(&op.args)?;
    let directives_json = op_directives_json(op)?;
    let schema_sha = ir.schema_sha256.as_deref().unwrap_or("");
    let codec_id = ir.codec_id.as_deref().unwrap_or(DEFAULT_CODEC_ID);
    let registry_version = ir.registry_version.unwrap_or(DEFAULT_REGISTRY_VERSION);
    let observer_plan_label = format!("observer:query/{schema_sha}/{}/{}", op.op_id, op.name);
    let observer_preimage = format!(
        concat!(
            "echo-wesley-query-observer-artifact/v1\n",
            "schema_sha256={schema_sha}\n",
            "codec_id={codec_id}\n",
            "registry_version={registry_version}\n",
            "op_id={op_id}\n",
            "op_name={op_name}\n",
            "result_type={result_type}\n",
            "args={args_json}\n",
            "directives_json={directives_json}\n",
            "generated_rust_artifact_hash={generated_rust_artifact_hash}\n",
        ),
        schema_sha = schema_sha,
        codec_id = codec_id,
        registry_version = registry_version,
        op_id = op.op_id,
        op_name = op.name,
        result_type = op.result_type,
        args_json = args_json,
        directives_json = directives_json,
        generated_rust_artifact_hash = generated_rust_artifact_hash,
    );
    let artifact_hash = blake3_hash(observer_preimage.as_bytes());
    let artifact_hash_hex = hash_hex(artifact_hash);
    Ok(GeneratedObserverIdentity {
        plan_id: blake3_hash(
            format!("echo-wesley-query-observer-plan-id/v1\n{observer_plan_label}\n").as_bytes(),
        ),
        artifact_hash,
        schema_hash: schema_hash_bytes(schema_sha),
        state_schema_hash: blake3_hash(
            format!(
                "echo-wesley-query-observer-state-schema/v1\n{}\n{}\n{}",
                schema_sha,
                op.op_id,
                artifact_hash_hex.as_str()
            )
            .as_bytes(),
        ),
        update_law_hash: blake3_hash(
            format!(
                "echo-wesley-query-observer-update-law/v1\n{}\n{}\n{}",
                schema_sha,
                op.op_id,
                artifact_hash_hex.as_str()
            )
            .as_bytes(),
        ),
        emission_law_hash: blake3_hash(
            format!(
                "echo-wesley-query-observer-emission-law/v1\n{}\n{}\n{}",
                schema_sha,
                op.op_id,
                artifact_hash_hex.as_str()
            )
            .as_bytes(),
        ),
    })
}

fn op_footprint_certificate(
    ir: &WesleyIR,
    op: &ir::OpDefinition,
    generated_rust_artifact_hash: &str,
) -> Result<Option<GeneratedFootprintCertificate>> {
    let Some(footprint) = op.directives.get("wes_footprint") else {
        return Ok(None);
    };

    let reads = footprint_string_items(footprint, "reads", &op.name)?;
    let writes = footprint_string_items(footprint, "writes", &op.name)?;
    let reads_json = serde_json::to_string(&reads)?;
    let writes_json = serde_json::to_string(&writes)?;
    let args_json = serde_json::to_string(&op.args)?;
    let directives_json = op_directives_json(op)?;
    let schema_sha = ir.schema_sha256.as_deref().unwrap_or("");
    let codec_id = ir.codec_id.as_deref().unwrap_or(DEFAULT_CODEC_ID);
    let registry_version = ir.registry_version.unwrap_or(DEFAULT_REGISTRY_VERSION);
    let kind = match op.kind {
        OpKind::Query => "QUERY",
        OpKind::Mutation => "MUTATION",
    };

    let artifact_preimage = format!(
        concat!(
            "echo-wesley-footprint-artifact/v1\n",
            "schema_sha256={schema_sha}\n",
            "codec_id={codec_id}\n",
            "registry_version={registry_version}\n",
            "op_kind={kind}\n",
            "op_id={op_id}\n",
            "op_name={op_name}\n",
            "result_type={result_type}\n",
            "args={args_json}\n",
            "generated_rust_artifact_hash={generated_rust_artifact_hash}\n",
            "reads={reads_json}\n",
            "writes={writes_json}\n",
        ),
        schema_sha = schema_sha,
        codec_id = codec_id,
        registry_version = registry_version,
        kind = kind,
        op_id = op.op_id,
        op_name = op.name,
        result_type = op.result_type,
        args_json = args_json,
        generated_rust_artifact_hash = generated_rust_artifact_hash,
        reads_json = reads_json,
        writes_json = writes_json,
    );
    let artifact_hash_hex = blake3_hex(artifact_preimage.as_bytes());
    let certificate_preimage = format!(
        concat!(
            "echo-wesley-footprint-certificate/v1\n",
            "generator=echo-wesley-gen\n",
            "generator_version={generator_version}\n",
            "artifact_hash={artifact_hash_hex}\n",
            "directives_json={directives_json}\n",
        ),
        generator_version = env!("CARGO_PKG_VERSION"),
        artifact_hash_hex = artifact_hash_hex,
        directives_json = directives_json,
    );
    let certificate_hash_hex = blake3_hex(certificate_preimage.as_bytes());

    Ok(Some(GeneratedFootprintCertificate {
        reads,
        writes,
        artifact_hash_hex,
        certificate_hash_hex,
    }))
}

fn footprint_string_items(
    footprint: &serde_json::Value,
    key: &str,
    op_name: &str,
) -> Result<Vec<String>> {
    let Some(value) = footprint_argument_value(footprint, key) else {
        return Ok(Vec::new());
    };
    let serde_json::Value::Array(items) = value else {
        bail!("wes_footprint.{key} for operation `{op_name}` must be an array of strings");
    };

    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let Some(item) = item.as_str() else {
            bail!("wes_footprint.{key} for operation `{op_name}` must contain only strings");
        };
        values.push(item.to_string());
    }
    values.sort();
    values.dedup();
    Ok(values)
}

fn footprint_argument_value<'a>(
    footprint: &'a serde_json::Value,
    key: &str,
) -> Option<&'a serde_json::Value> {
    footprint.get(key).or_else(|| {
        footprint
            .get("arguments")
            .and_then(|arguments| arguments.get(key))
    })
}

fn blake3_hex(input: &[u8]) -> String {
    blake3::hash(input).to_hex().to_string()
}

fn blake3_hash(input: &[u8]) -> [u8; 32] {
    blake3::hash(input).into()
}

fn schema_hash_bytes(schema_sha: &str) -> [u8; 32] {
    parse_hex_hash(schema_sha).unwrap_or_else(|| {
        blake3_hash(format!("echo-wesley-schema-hash-fallback/v1\n{schema_sha}\n").as_bytes())
    })
}

fn parse_hex_hash(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        bytes[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(bytes)
}

fn hash_hex(hash: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in hash {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn byte_array_literal(bytes: [u8; 32]) -> TokenStream {
    let elements = bytes.iter().copied();
    quote! { [#(#elements),*] }
}

fn op_const_name(name: &str, op_id: u32) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        return format!("OP_ID_{op_id}");
    }
    format!("OP_{out}")
}

/// Convert a Wesley operation name to a Rust PascalCase stem.
///
/// Existing alphanumeric casing is preserved between separators so acronym-heavy
/// names such as `XMLParser` remain `XMLParser` instead of being normalized to
/// title case.
fn to_pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = true;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                out.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(c);
            }
        } else {
            capitalize_next = true;
        }
    }
    if out.is_empty() {
        "Op".to_string()
    } else {
        out
    }
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_separator = true;
    for (index, c) in name.chars().enumerate() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && index > 0 && !previous_was_separator {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            out.push('_');
            previous_was_separator = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "op".to_string()
    } else {
        out
    }
}

fn optic_mutation_helper_stem(name: &str) -> String {
    let stem = to_snake_case(name);
    if stem == "set" || stem.starts_with("set_") {
        format!("propose_{stem}")
    } else {
        stem
    }
}

fn validate_version(ir: &WesleyIR) -> Result<()> {
    const SUPPORTED: &str = "echo-ir/v1";
    match ir.ir_version.as_deref() {
        Some(SUPPORTED) => Ok(()),
        Some(other) => anyhow::bail!(
            "Unsupported ir_version '{other}'; expected '{SUPPORTED}'. Please regenerate IR with a compatible generator."
        ),
        None => anyhow::bail!(
            "Missing ir_version; expected '{SUPPORTED}'. Regenerate IR with a current @wesley/generator-echo."
        ),
    }
}

fn validate_generated_item_names(ir: &WesleyIR) -> Result<()> {
    let mut top_level_items = BTreeMap::new();
    let mut helper_items = BTreeMap::new();

    record_generated_item(
        &mut top_level_items,
        "SCHEMA_SHA256",
        "generated schema hash constant",
    )?;
    record_generated_item(
        &mut top_level_items,
        "ECHO_CONTRACT_ABI_VERSION",
        "generated Echo contract ABI version constant",
    )?;
    record_generated_item(
        &mut top_level_items,
        "CODEC_ID",
        "generated codec id constant",
    )?;
    record_generated_item(
        &mut top_level_items,
        "REGISTRY_VERSION",
        "generated registry version constant",
    )?;
    record_generated_item(
        &mut top_level_items,
        "WESLEY_GENERATOR_VERSION",
        "generated Wesley generator version constant",
    )?;
    record_generated_item(
        &mut top_level_items,
        "CONTRACT_HOST_HELPER_API_VERSION",
        "generated contract-host helper API version constant",
    )?;

    for type_def in &ir.types {
        match type_def.kind {
            TypeKind::Enum => {
                record_generated_item(
                    &mut top_level_items,
                    type_def.name.as_str(),
                    format!("enum type `{}`", type_def.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("ENUM_{}_VALUES", type_def.name.to_ascii_uppercase()),
                    format!("enum `{}` values constant", type_def.name),
                )?;
            }
            TypeKind::Object | TypeKind::InputObject => {
                record_generated_item(
                    &mut top_level_items,
                    type_def.name.as_str(),
                    format!("object type `{}`", type_def.name),
                )?;
                if type_def.kind == TypeKind::Object {
                    record_generated_item(
                        &mut top_level_items,
                        format!("OBJ_{}_FIELDS", type_def.name.to_ascii_uppercase()),
                        format!("object `{}` fields constant", type_def.name),
                    )?;
                }
            }
            TypeKind::Scalar | TypeKind::Interface | TypeKind::Union => {}
        }
    }

    if !ir.ops.is_empty() {
        for (name, source) in [
            ("ENUMS", "generated enum registry"),
            ("OBJECTS", "generated object registry"),
            ("OPS", "generated operation registry"),
            ("op_by_id", "generated operation lookup function"),
            ("op_by_name", "generated operation lookup function"),
            ("GeneratedRegistry", "generated registry provider type"),
            ("REGISTRY", "generated registry provider value"),
        ] {
            record_generated_item(&mut top_level_items, name, source)?;
        }

        record_generated_item(
            &mut top_level_items,
            "__echo_wesley_generated",
            "generated operation helper namespace",
        )?;
    }

    if ir.ops.iter().any(|op| op.kind == OpKind::Mutation) {
        record_generated_item(
            &mut helper_items,
            "GeneratedIntentError",
            "generated intent helper error",
        )?;
    }
    if ir.ops.iter().any(|op| op.kind == OpKind::Query) {
        record_generated_item(
            &mut helper_items,
            "generated_vars_digest",
            "generated optic query vars digest helper",
        )?;
    }

    for op in &ir.ops {
        let kind = op_kind_name(&op.kind);
        let const_name = op_const_name(&op.name, op.op_id);
        let helper_name = to_snake_case(&op.name);
        let optic_mutation_helper_name = optic_mutation_helper_stem(&op.name);

        record_generated_item(
            &mut top_level_items,
            const_name.as_str(),
            format!("{kind} operation `{}` id constant", op.name),
        )?;
        record_generated_item(
            &mut top_level_items,
            format!("{const_name}_ARGS"),
            format!("{kind} operation `{}` args constant", op.name),
        )?;
        record_generated_item(
            &mut helper_items,
            format!("{}Vars", to_pascal_case(&op.name)),
            format!("{kind} operation `{}` vars type", op.name),
        )?;
        record_generated_item(
            &mut helper_items,
            format!("encode_{helper_name}_vars"),
            format!("{kind} operation `{}` vars encoder", op.name),
        )?;
        record_generated_item(
            &mut top_level_items,
            format!("encode_{helper_name}_vars"),
            format!("{kind} operation `{}` vars encoder re-export", op.name),
        )?;

        match op.kind {
            OpKind::Mutation => {
                record_generated_item(
                    &mut helper_items,
                    format!("pack_{helper_name}_intent"),
                    format!("mutation operation `{}` EINT helper", op.name),
                )?;
                record_generated_item(
                    &mut helper_items,
                    format!("pack_{helper_name}_intent_raw_vars"),
                    format!("mutation operation `{}` raw EINT helper", op.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("pack_{helper_name}_intent"),
                    format!("mutation operation `{}` EINT helper re-export", op.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("pack_{helper_name}_intent_raw_vars"),
                    format!("mutation operation `{}` raw EINT helper re-export", op.name),
                )?;
                record_generated_item(
                    &mut helper_items,
                    format!("{optic_mutation_helper_name}_dispatch_optic_intent_request"),
                    format!("mutation operation `{}` optic dispatch helper", op.name),
                )?;
                record_generated_item(
                    &mut helper_items,
                    format!("{optic_mutation_helper_name}_dispatch_optic_intent_request_raw_vars"),
                    format!("mutation operation `{}` raw optic dispatch helper", op.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("{optic_mutation_helper_name}_dispatch_optic_intent_request"),
                    format!(
                        "mutation operation `{}` optic dispatch helper re-export",
                        op.name
                    ),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("{optic_mutation_helper_name}_dispatch_optic_intent_request_raw_vars"),
                    format!(
                        "mutation operation `{}` raw optic dispatch helper re-export",
                        op.name
                    ),
                )?;
            }
            OpKind::Query => {
                record_generated_item(
                    &mut helper_items,
                    format!("{helper_name}_observation_request"),
                    format!("query operation `{}` observation helper", op.name),
                )?;
                record_generated_item(
                    &mut helper_items,
                    format!("{helper_name}_observation_request_raw_vars"),
                    format!("query operation `{}` raw observation helper", op.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("{helper_name}_observation_request"),
                    format!("query operation `{}` observation helper re-export", op.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("{helper_name}_observation_request_raw_vars"),
                    format!(
                        "query operation `{}` raw observation helper re-export",
                        op.name
                    ),
                )?;
                record_generated_item(
                    &mut helper_items,
                    format!("{helper_name}_observe_optic_request"),
                    format!("query operation `{}` optic observe helper", op.name),
                )?;
                record_generated_item(
                    &mut helper_items,
                    format!("{helper_name}_observe_optic_request_raw_vars"),
                    format!("query operation `{}` raw optic observe helper", op.name),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("{helper_name}_observe_optic_request"),
                    format!(
                        "query operation `{}` optic observe helper re-export",
                        op.name
                    ),
                )?;
                record_generated_item(
                    &mut top_level_items,
                    format!("{helper_name}_observe_optic_request_raw_vars"),
                    format!(
                        "query operation `{}` raw optic observe helper re-export",
                        op.name
                    ),
                )?;
            }
        }
    }

    Ok(())
}

fn record_generated_item(
    items: &mut BTreeMap<String, String>,
    name: impl Into<String>,
    source: impl Into<String>,
) -> Result<()> {
    let name = name.into();
    let source = source.into();
    if let Some(existing_source) = items.get(&name) {
        anyhow::bail!(
            "generated Rust item name collision for `{name}`: {existing_source} conflicts with {source}"
        );
    }
    items.insert(name, source);
    Ok(())
}

fn op_kind_name(kind: &OpKind) -> &'static str {
    match kind {
        OpKind::Query => "query",
        OpKind::Mutation => "mutation",
    }
}

/// Generate a single encode statement for an ArgDefinition inside a Vars struct (`self.field`).
///
/// User-defined types use `super::TypeName` path convention.
fn encode_arg_stmt(arg: &ir::ArgDefinition, args: &Args) -> TokenStream {
    let field_name = safe_ident(&arg.name);
    if arg.list {
        let inner_encode = scalar_list_element_encoder(&arg.type_name, args);
        if arg.required {
            return quote! {
                w.write_list(&self.#field_name, #inner_encode)?;
            };
        }
        // Nullable list: Vars field is Option<Vec<_>>; must wrap with write_option.
        return quote! {
            w.write_option(self.#field_name.as_ref(), |w, v| {
                w.write_list(v, #inner_encode)
            })?;
        };
    }
    // no_std maps the GraphQL ID scalar to [u8; 32]; encode the raw bytes
    // instead of treating it like String.
    let id_is_bytes = args.no_std && arg.type_name == "ID";
    if arg.required {
        match arg.type_name.as_str() {
            "Boolean" => quote! { w.write_bool(self.#field_name); },
            "Int" => quote! { w.write_i32_le(self.#field_name); },
            "Float" => quote! { w.write_f32_le(self.#field_name); },
            "ID" if id_is_bytes => quote! { w.write_bytes(&self.#field_name); },
            "String" | "ID" => quote! { w.write_string(&self.#field_name, usize::MAX)?; },
            _ => quote! { self.#field_name.encode(w)?; },
        }
    } else {
        match arg.type_name.as_str() {
            "Boolean" => quote! {
                w.write_option(self.#field_name, |w, v| { w.write_bool(v); Ok(()) })?;
            },
            "Int" => quote! {
                w.write_option(self.#field_name, |w, v| { w.write_i32_le(v); Ok(()) })?;
            },
            "Float" => quote! {
                w.write_option(self.#field_name, |w, v| { w.write_f32_le(v); Ok(()) })?;
            },
            "ID" if id_is_bytes => quote! {
                w.write_option(self.#field_name.as_ref(), |w, v| { w.write_bytes(v); Ok(()) })?;
            },
            "String" | "ID" => quote! {
                w.write_option(self.#field_name.as_deref(), |w, v| w.write_string(v, usize::MAX))?;
            },
            _ => quote! {
                w.write_option(self.#field_name.as_ref(), |w, v| v.encode(w))?;
            },
        }
    }
}

/// Generate the field initializer for an ArgDefinition inside a Vars Decode impl.
///
/// User-defined types use `super::TypeName::decode(r)?` path convention.
fn decode_arg_expr(arg: &ir::ArgDefinition, args: &Args) -> TokenStream {
    let field_name = safe_ident(&arg.name);
    if arg.list {
        // Vars Decode impl sits inside the private __echo_wesley_generated
        // module; user-defined element types live in the parent module and
        // need `super::` qualification.
        let inner_decode =
            scalar_list_element_decoder(&arg.type_name, args, /* super_qualified */ true);
        if arg.required {
            return quote! {
                #field_name: r.read_list(#inner_decode)?
            };
        }
        // Nullable list: field is Option<Vec<_>>; wrap with read_option.
        return quote! {
            #field_name: r.read_option(|r| r.read_list(#inner_decode))?
        };
    }
    let id_is_bytes = args.no_std && arg.type_name == "ID";
    if arg.required {
        let expr = match arg.type_name.as_str() {
            "Boolean" => quote! { r.read_bool()? },
            "Int" => quote! { r.read_i32_le()? },
            "Float" => quote! { r.read_f32_le()? },
            "ID" if id_is_bytes => quote! { r.read_byte_array::<32>()? },
            "String" | "ID" => quote! { r.read_string(usize::MAX)? },
            _ => {
                let ty = safe_ident(&arg.type_name);
                quote! { super::#ty::decode(r)? }
            }
        };
        quote! { #field_name: #expr }
    } else {
        let expr = match arg.type_name.as_str() {
            "Boolean" => quote! { r.read_option(|r| r.read_bool())? },
            "Int" => quote! { r.read_option(|r| r.read_i32_le())? },
            "Float" => quote! { r.read_option(|r| r.read_f32_le())? },
            "ID" if id_is_bytes => quote! { r.read_option(|r| r.read_byte_array::<32>())? },
            "String" | "ID" => quote! { r.read_option(|r| r.read_string(usize::MAX))? },
            _ => {
                let ty = safe_ident(&arg.type_name);
                quote! { r.read_option(|r| super::#ty::decode(r))? }
            }
        };
        quote! { #field_name: #expr }
    }
}

/// Generate a single encode statement for a FieldDefinition on a named struct (`self.field`).
///
/// Uses the LE binary codec primitives from `echo_wasm_abi::codec`.
fn encode_field_stmt(field: &ir::FieldDefinition, args: &Args) -> TokenStream {
    let field_name = safe_ident(&field.name);
    if field.list {
        let inner_encode = scalar_list_element_encoder(&field.type_name, args);
        if field.required {
            // [T!]! required list
            return quote! {
                w.write_list(&self.#field_name, #inner_encode)?;
            };
        }
        // [T!] nullable list: field is Option<Vec<_>>; wrap with write_option.
        return quote! {
            w.write_option(self.#field_name.as_ref(), |w, v| {
                w.write_list(v, #inner_encode)
            })?;
        };
    }
    let id_is_bytes = args.no_std && field.type_name == "ID";
    if field.required {
        // Required (non-nullable) scalar or user type
        match field.type_name.as_str() {
            "Boolean" => quote! { w.write_bool(self.#field_name); },
            "Int" => quote! { w.write_i32_le(self.#field_name); },
            "Float" => quote! { w.write_f32_le(self.#field_name); },
            "ID" if id_is_bytes => quote! { w.write_bytes(&self.#field_name); },
            "String" | "ID" => quote! { w.write_string(&self.#field_name, usize::MAX)?; },
            _ => {
                // User-defined type — delegate to its Encode impl
                quote! { self.#field_name.encode(w)?; }
            }
        }
    } else {
        // Nullable (Option<T>)
        match field.type_name.as_str() {
            "Boolean" => quote! {
                w.write_option(self.#field_name, |w, v| { w.write_bool(v); Ok(()) })?;
            },
            "Int" => quote! {
                w.write_option(self.#field_name, |w, v| { w.write_i32_le(v); Ok(()) })?;
            },
            "Float" => quote! {
                w.write_option(self.#field_name, |w, v| { w.write_f32_le(v); Ok(()) })?;
            },
            "ID" if id_is_bytes => quote! {
                w.write_option(self.#field_name.as_ref(), |w, v| { w.write_bytes(v); Ok(()) })?;
            },
            "String" | "ID" => quote! {
                w.write_option(self.#field_name.as_deref(), |w, v| w.write_string(v, usize::MAX))?;
            },
            _ => {
                // User-defined type — delegate to its Encode impl
                quote! {
                    w.write_option(self.#field_name.as_ref(), |w, v| v.encode(w))?;
                }
            }
        }
    }
}

/// Generate the field initializer for a FieldDefinition inside a Decode impl (`field_name: ...`).
fn decode_field_expr(field: &ir::FieldDefinition, args: &Args) -> TokenStream {
    let field_name = safe_ident(&field.name);
    if field.list {
        // FieldDefinition Decode impls live alongside the user types
        // themselves; no `super::` qualification needed.
        let inner_decode =
            scalar_list_element_decoder(&field.type_name, args, /* super_qualified */ false);
        if field.required {
            return quote! {
                #field_name: r.read_list(#inner_decode)?
            };
        }
        return quote! {
            #field_name: r.read_option(|r| r.read_list(#inner_decode))?
        };
    }
    let id_is_bytes = args.no_std && field.type_name == "ID";
    if field.required {
        let expr = match field.type_name.as_str() {
            "Boolean" => quote! { r.read_bool()? },
            "Int" => quote! { r.read_i32_le()? },
            "Float" => quote! { r.read_f32_le()? },
            "ID" if id_is_bytes => quote! { r.read_byte_array::<32>()? },
            "String" | "ID" => quote! { r.read_string(usize::MAX)? },
            _ => {
                let ty = safe_ident(&field.type_name);
                quote! { #ty::decode(r)? }
            }
        };
        quote! { #field_name: #expr }
    } else {
        let expr = match field.type_name.as_str() {
            "Boolean" => quote! { r.read_option(|r| r.read_bool())? },
            "Int" => quote! { r.read_option(|r| r.read_i32_le())? },
            "Float" => quote! { r.read_option(|r| r.read_f32_le())? },
            "ID" if id_is_bytes => quote! { r.read_option(|r| r.read_byte_array::<32>())? },
            "String" | "ID" => quote! { r.read_option(|r| r.read_string(usize::MAX))? },
            _ => {
                let ty = safe_ident(&field.type_name);
                quote! { r.read_option(|r| #ty::decode(r))? }
            }
        };
        quote! { #field_name: #expr }
    }
}

/// Generate a list element encoder closure for `write_list`.
fn scalar_list_element_encoder(type_name: &str, args: &Args) -> TokenStream {
    // no_std maps GraphQL ID to [u8; 32]; encoder must emit raw bytes, not
    // treat the element as &String.
    let id_is_bytes = args.no_std && type_name == "ID";
    match type_name {
        "Boolean" => quote! { |w, v| { w.write_bool(*v); Ok(()) } },
        "Int" => quote! { |w, v| { w.write_i32_le(*v); Ok(()) } },
        "Float" => quote! { |w, v| { w.write_f32_le(*v); Ok(()) } },
        "ID" if id_is_bytes => quote! { |w, v| { w.write_bytes(v); Ok(()) } },
        "String" | "ID" => quote! { |w, v| w.write_string(v.as_str(), usize::MAX) },
        _ => quote! { |w, v| v.encode(w) },
    }
}

/// Generate a list element decoder closure for `read_list`.
///
/// `super_qualified` matters for user-defined element types: Vars Decode
/// impls are emitted inside the `__echo_wesley_generated` private module
/// (so user types live in the parent and need `super::`), but
/// FieldDefinition Decode impls are emitted alongside the user types
/// themselves (so `super::` would over-qualify). The non-list scalar
/// decoders pick the right form already; this helper had been emitting a
/// bare `#ty::decode(r)` regardless, so list-of-user-types under Vars
/// failed to compile (`tags: [Tag!]!` → `Tag::decode` unresolved).
fn scalar_list_element_decoder(type_name: &str, args: &Args, super_qualified: bool) -> TokenStream {
    // no_std maps GraphQL ID to [u8; 32]; decoder must read a fixed-size
    // byte array, not a length-prefixed String.
    let id_is_bytes = args.no_std && type_name == "ID";
    match type_name {
        "Boolean" => quote! { |r| r.read_bool() },
        "Int" => quote! { |r| r.read_i32_le() },
        "Float" => quote! { |r| r.read_f32_le() },
        "ID" if id_is_bytes => quote! { |r| r.read_byte_array::<32>() },
        "String" | "ID" => quote! { |r| r.read_string(usize::MAX) },
        _ => {
            let ty = safe_ident(type_name);
            if super_qualified {
                quote! { |r| super::#ty::decode(r) }
            } else {
                quote! { |r| #ty::decode(r) }
            }
        }
    }
}

/// Map a GraphQL base type name to a Rust type used in generated DTOs.
///
/// GraphQL `Float` intentionally maps to `f32` (not `f64`) so generated types
/// integrate cleanly with Echo’s deterministic scalar foundation.
fn map_type(gql_type: &str, args: &Args) -> TokenStream {
    match gql_type {
        "Boolean" => quote! { bool },
        "String" => quote! { String },
        "Int" => quote! { i32 },
        "Float" => quote! { f32 },
        "ID" => {
            if args.no_std {
                quote! { [u8; 32] }
            } else {
                quote! { String }
            }
        }
        other => {
            let ident = safe_ident(other);
            quote! { #ident }
        }
    }
}

/// Map a GraphQL base type name for use inside the generated helper module.
fn map_helper_type(gql_type: &str, args: &Args) -> TokenStream {
    match gql_type {
        "Boolean" => quote! { bool },
        "String" => quote! { String },
        "Int" => quote! { i32 },
        "Float" => quote! { f32 },
        "ID" => {
            if args.no_std {
                quote! { [u8; 32] }
            } else {
                quote! { String }
            }
        }
        other => {
            let ident = safe_ident(other);
            quote! { super::#ident }
        }
    }
}

#[cfg(test)]
mod stable_op_id_pinned {
    use super::stable_op_id;
    use wesley_core::OperationType;

    /// These u32 outputs are the cross-language contract surface. They MUST
    /// stay bytewise identical to `wesley_core::stable_op_id` and to every
    /// TypeScript / WASM consumer that routes EINT envelopes by op id. If a
    /// value changes here, every contract that uses that op id breaks.
    #[test]
    fn rope_operation_op_ids_are_pinned() {
        assert_eq!(
            stable_op_id(&OperationType::Mutation, "createBufferWorldline"),
            2_519_122_874
        );
        assert_eq!(
            stable_op_id(&OperationType::Mutation, "replaceRangeAsTick"),
            3_329_158_538
        );
        assert_eq!(
            stable_op_id(&OperationType::Mutation, "createCheckpoint"),
            3_744_251_216
        );
        assert_eq!(
            stable_op_id(&OperationType::Query, "worldlineSnapshot"),
            3_219_688_859
        );
        assert_eq!(
            stable_op_id(&OperationType::Query, "textWindow"),
            2_414_231_278
        );
    }
}
