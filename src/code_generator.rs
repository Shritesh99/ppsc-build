use std::collections::{HashMap, HashSet};
use std::iter;

use itertools::{Either, Itertools};
use log::debug;
use multimap::MultiMap;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::source_code_info::Location;
use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
};

use crate::Config;
use crate::ast::{Comments, Method, Service};
use crate::context::Context;
use crate::ident::{strip_enum_prefix, to_snake, to_upper_camel};

mod c_escaping;

mod syntax;
use syntax::Syntax;

/// State object for the code generation process on a single input file.
pub struct CodeGenerator<'a, 'b> {
    context: &'a mut Context<'b>,
    package: String,
    type_path: Vec<String>,
    source_info: Option<SourceCodeInfo>,
    syntax: Syntax,
    depth: u8,
    path: Vec<i32>,
    buf: &'a mut String,
}

fn push_indent(buf: &mut String, depth: u8) {
    for _ in 0..depth {
        buf.push_str("    ");
    }
}

struct Field {
    descriptor: FieldDescriptorProto,
    path_index: i32,
}

impl Field {
    fn new(descriptor: FieldDescriptorProto, path_index: i32) -> Self {
        Self {
            descriptor,
            path_index,
        }
    }

    fn rust_name(&self) -> String {
        to_snake(self.descriptor.name())
    }
}

struct OneofField {
    descriptor: OneofDescriptorProto,
    fields: Vec<Field>,
    path_index: i32,
}

impl OneofField {
    fn new(descriptor: OneofDescriptorProto, fields: Vec<Field>, path_index: i32) -> Self {
        Self {
            descriptor,
            fields,
            path_index,
        }
    }

    fn rust_name(&self) -> String {
        to_snake(self.descriptor.name())
    }
}

impl<'b> CodeGenerator<'_, 'b> {
    fn config(&self) -> &Config {
        self.context.config()
    }

    pub(crate) fn generate(context: &mut Context<'b>, file: FileDescriptorProto, buf: &mut String) {
        let source_info = file.source_code_info.map(|mut s| {
            s.location.retain(|loc| {
                let len = loc.path.len();
                len > 0 && len % 2 == 0
            });
            s.location.sort_by(|a, b| a.path.cmp(&b.path));
            s
        });

        let mut code_gen = CodeGenerator {
            context,
            package: file.package.unwrap_or_default(),
            type_path: Vec::new(),
            source_info,
            syntax: file.syntax.as_deref().into(),
            depth: 0,
            path: Vec::new(),
            buf,
        };

        debug!(
            "file: {:?}, package: {:?}",
            file.name.as_ref().unwrap(),
            code_gen.package
        );
        code_gen.buf.push_str("extern crate alloc;\n");
        code_gen.push_indent();
        code_gen
            .buf
            .push_str("use parity_scale_codec::{Encode, Decode};\n\n");

        code_gen.path.push(4);
        for (idx, message) in file.message_type.into_iter().enumerate() {
            code_gen.path.push(idx as i32);
            code_gen.append_message(message);
            code_gen.path.pop();
        }
        code_gen.path.pop();

        code_gen.path.push(5);
        for (idx, desc) in file.enum_type.into_iter().enumerate() {
            code_gen.path.push(idx as i32);
            code_gen.append_enum(desc);
            code_gen.path.pop();
        }
        code_gen.path.pop();

        if code_gen.context.service_generator_mut().is_some() {
            code_gen.path.push(6);
            for (idx, service) in file.service.into_iter().enumerate() {
                code_gen.path.push(idx as i32);
                code_gen.push_service(service);
                code_gen.path.pop();
            }

            if let Some(service_generator) = code_gen.context.service_generator_mut() {
                service_generator.finalize(code_gen.buf);
            }

            code_gen.path.pop();
        }
    }

    fn append_message(&mut self, message: DescriptorProto) {
        debug!("  message: {:?}", message.name());

        let message_name = message.name().to_string();
        let fq_message_name = self.fq_name(&message_name);

        // Skip external types.
        if self
            .context
            .resolve_extern_ident(&fq_message_name)
            .is_some()
        {
            return;
        }

        // Split the nested message types into a vector of normal nested message types, and a map
        // of the map field entry types. The path index of the nested message types is preserved so
        // that comments can be retrieved.
        type NestedTypes = Vec<(DescriptorProto, usize)>;
        type MapTypes = HashMap<String, (FieldDescriptorProto, FieldDescriptorProto)>;
        let (nested_types, map_types): (NestedTypes, MapTypes) = message
            .nested_type
            .into_iter()
            .enumerate()
            .partition_map(|(idx, nested_type)| {
                if nested_type
                    .options
                    .as_ref()
                    .and_then(|options| options.map_entry)
                    .unwrap_or(false)
                {
                    let key = nested_type.field[0].clone();
                    let value = nested_type.field[1].clone();
                    assert_eq!("key", key.name());
                    assert_eq!("value", value.name());

                    let name = format!("{}.{}", &fq_message_name, nested_type.name());
                    Either::Right((name, (key, value)))
                } else {
                    Either::Left((nested_type, idx))
                }
            });

        // Split the fields into a vector of the normal fields, and oneof fields.
        // Path indexes are preserved so that comments can be retrieved.
        type OneofFieldsByIndex = MultiMap<i32, Field>;
        let (fields, mut oneof_map): (Vec<Field>, OneofFieldsByIndex) = message
            .field
            .into_iter()
            .enumerate()
            .partition_map(|(idx, proto)| {
                let idx = idx as i32;
                if proto.proto3_optional.unwrap_or(false) {
                    Either::Left(Field::new(proto, idx))
                } else if let Some(oneof_index) = proto.oneof_index {
                    Either::Right((oneof_index, Field::new(proto, idx)))
                } else {
                    Either::Left(Field::new(proto, idx))
                }
            });
        // Optional fields create a synthetic oneof that we want to skip
        let oneof_fields: Vec<OneofField> = message
            .oneof_decl
            .into_iter()
            .enumerate()
            .filter_map(move |(idx, proto)| {
                let idx = idx as i32;
                oneof_map
                    .remove(&idx)
                    .map(|fields| OneofField::new(proto, fields, idx))
            })
            .collect();

        self.append_doc(&fq_message_name, None);
        self.append_type_attributes(&fq_message_name);
        self.append_message_attributes(&fq_message_name);
        self.push_indent();
        self.buf.push_str(&format!("#[derive(Encode, Decode)]\n"));
        // self.append_skip_debug(&fq_message_name);
        self.push_indent();
        self.buf.push_str("pub struct ");
        self.buf.push_str(&to_upper_camel(&message_name));
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for field in &fields {
            self.path.push(field.path_index);
            match field
                .descriptor
                .type_name
                .as_ref()
                .and_then(|type_name| map_types.get(type_name))
            {
                Some((key, value)) => self.append_map_field(&fq_message_name, field, key, value),
                None => self.append_field(&fq_message_name, field),
            }
            self.path.pop();
        }
        self.path.pop();

        self.path.push(8);
        for oneof in &oneof_fields {
            self.path.push(oneof.path_index);
            self.append_oneof_field(&message_name, &fq_message_name, oneof);
            self.path.pop();
        }
        self.path.pop();

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n");

        if !message.enum_type.is_empty() || !nested_types.is_empty() || !oneof_fields.is_empty() {
            self.push_mod(&message_name);
            self.path.push(3);
            self.push_indent();
            self.buf.push_str("use super::*;\n\n");
            for (nested_type, idx) in nested_types {
                self.path.push(idx as i32);
                self.append_message(nested_type);
                self.path.pop();
            }
            self.path.pop();

            self.path.push(4);
            for (idx, nested_enum) in message.enum_type.into_iter().enumerate() {
                self.path.push(idx as i32);
                self.append_enum(nested_enum);
                self.path.pop();
            }
            self.path.pop();

            for oneof in &oneof_fields {
                self.append_oneof(&fq_message_name, oneof);
            }

            self.pop_mod();
        }
    }

    fn append_type_attributes(&mut self, fq_message_name: &str) {
        assert_eq!(b'.', fq_message_name.as_bytes()[0]);
        for attribute in self.context.type_attributes(fq_message_name) {
            push_indent(self.buf, self.depth);
            self.buf.push_str(attribute);
            self.buf.push('\n');
        }
    }

    fn append_message_attributes(&mut self, fq_message_name: &str) {
        assert_eq!(b'.', fq_message_name.as_bytes()[0]);
        for attribute in self.context.message_attributes(fq_message_name) {
            push_indent(self.buf, self.depth);
            self.buf.push_str(attribute);
            self.buf.push('\n');
        }
    }

    fn append_enum_attributes(&mut self, fq_message_name: &str) {
        assert_eq!(b'.', fq_message_name.as_bytes()[0]);
        for attribute in self.context.enum_attributes(fq_message_name) {
            push_indent(self.buf, self.depth);
            self.buf.push_str(attribute);
            self.buf.push('\n');
        }
    }

    fn append_field_attributes(&mut self, fq_message_name: &str, field_name: &str) {
        assert_eq!(b'.', fq_message_name.as_bytes()[0]);
        for attribute in self.context.field_attributes(fq_message_name, field_name) {
            push_indent(self.buf, self.depth);
            self.buf.push_str(attribute);
            self.buf.push('\n');
        }
    }

    fn append_field(&mut self, fq_message_name: &str, field: &Field) {
        let repeated = field.descriptor.label() == Label::Repeated;
        let optional = self.optional(&field.descriptor);
        let boxed = self
            .context
            .should_box_message_field(fq_message_name, &field.descriptor);
        let ty = self.resolve_type(&field.descriptor, fq_message_name);

        debug!(
            "    field: {:?}, type: {:?}, boxed: {}",
            field.descriptor.name(),
            ty,
            boxed
        );

        self.append_doc(fq_message_name, Some(field.descriptor.name()));

        self.append_field_attributes(fq_message_name, field.descriptor.name());
        self.push_indent();
        self.buf.push_str("pub ");
        self.buf.push_str(&field.rust_name());
        self.buf.push_str(": ");

        if repeated {
            self.buf.push_str(&format!("alloc::vec::Vec<"));
        } else if optional {
            self.buf.push_str("Option<");
        }
        if boxed {
            self.buf.push_str(&format!("alloc::boxed::Box<"));
        }
        self.buf.push_str(&ty);
        if boxed {
            self.buf.push('>');
        }
        if repeated || optional {
            self.buf.push('>');
        }
        self.buf.push_str(",\n");
    }

    fn append_map_field(
        &mut self,
        fq_message_name: &str,
        field: &Field,
        key: &FieldDescriptorProto,
        value: &FieldDescriptorProto,
    ) {
        let key_ty = self.resolve_type(key, fq_message_name);
        let value_ty = self.resolve_type(value, fq_message_name);

        debug!(
            "    map field: {:?}, key type: {:?}, value type: {:?}",
            field.descriptor.name(),
            key_ty,
            value_ty
        );

        self.append_doc(fq_message_name, Some(field.descriptor.name()));

        let map_type = self
            .context
            .map_type(fq_message_name, field.descriptor.name());
        self.append_field_attributes(fq_message_name, field.descriptor.name());
        self.push_indent();
        self.buf.push_str(&format!(
            "pub {}: {}<{}, {}>,\n",
            field.rust_name(),
            map_type.rust_type(),
            key_ty,
            value_ty
        ));
    }

    fn append_oneof_field(
        &mut self,
        message_name: &str,
        fq_message_name: &str,
        oneof: &OneofField,
    ) {
        let type_name = format!(
            "{}::{}",
            to_snake(message_name),
            to_upper_camel(oneof.descriptor.name())
        );
        self.append_doc(fq_message_name, None);
        self.push_indent();
        self.append_field_attributes(fq_message_name, oneof.descriptor.name());
        self.buf.push_str(&format!(
            "pub {}: Option<{}>,\n",
            oneof.rust_name(),
            type_name
        ));
    }

    fn append_oneof(&mut self, fq_message_name: &str, oneof: &OneofField) {
        self.path.push(8);
        self.path.push(oneof.path_index);
        self.append_doc(fq_message_name, None);
        self.path.pop();
        self.path.pop();

        let oneof_name = format!("{}.{}", fq_message_name, oneof.descriptor.name());
        self.append_type_attributes(&oneof_name);
        self.append_enum_attributes(&oneof_name);
        self.push_indent();
        self.push_indent();
        self.buf.push_str(&format!("#[derive(Encode, Decode)]\n"));
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&to_upper_camel(oneof.descriptor.name()));
        self.buf.push_str(" {\n");

        self.path.push(2);
        self.depth += 1;
        for field in &oneof.fields {
            self.path.push(field.path_index);
            self.append_doc(fq_message_name, Some(field.descriptor.name()));
            self.path.pop();

            self.push_indent();
            self.append_field_attributes(&oneof_name, field.descriptor.name());

            let ty = self.resolve_type(&field.descriptor, fq_message_name);

            let boxed = self.context.should_box_oneof_field(
                fq_message_name,
                oneof.descriptor.name(),
                &field.descriptor,
            );

            debug!(
                "    oneof: {:?}, type: {:?}, boxed: {}",
                field.descriptor.name(),
                ty,
                boxed
            );

            if boxed {
                self.buf.push_str(&format!(
                    "{}(alloc::boxed::Box<{}>),\n",
                    to_upper_camel(field.descriptor.name()),
                    ty
                ));
            } else {
                self.buf.push_str(&format!(
                    "{}({}),\n",
                    to_upper_camel(field.descriptor.name()),
                    ty
                ));
            }
        }
        self.depth -= 1;
        self.path.pop();

        self.push_indent();
        self.buf.push_str("}\n");
    }

    fn location(&self) -> Option<&Location> {
        let source_info = self.source_info.as_ref()?;
        let idx = source_info
            .location
            .binary_search_by_key(&&self.path[..], |location| &location.path[..])
            .unwrap();
        Some(&source_info.location[idx])
    }

    fn append_doc(&mut self, fq_name: &str, field_name: Option<&str>) {
        if !self.context.should_disable_comments(fq_name, field_name) {
            if let Some(comments) = self.location().map(Comments::from_location) {
                comments.append_with_indent(self.depth, self.buf);
            }
        }
    }

    fn append_enum(&mut self, desc: EnumDescriptorProto) {
        debug!("  enum: {:?}", desc.name());

        let proto_enum_name = desc.name();
        let enum_name = to_upper_camel(proto_enum_name);

        let enum_values = &desc.value;
        let fq_proto_enum_name = self.fq_name(proto_enum_name);

        if self
            .context
            .resolve_extern_ident(&fq_proto_enum_name)
            .is_some()
        {
            return;
        }

        self.append_doc(&fq_proto_enum_name, None);
        self.append_type_attributes(&fq_proto_enum_name);
        self.append_enum_attributes(&fq_proto_enum_name);
        self.push_indent();

        self.buf.push_str(&format!("#[derive(Encode, Decode)]\n"));
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&enum_name);
        self.buf.push_str(" {\n");

        let variant_mappings =
            build_enum_value_mappings(&enum_name, self.config().strip_enum_prefix, enum_values);

        self.depth += 1;
        self.path.push(2);
        for variant in variant_mappings.iter() {
            self.path.push(variant.path_idx as i32);

            self.append_doc(&fq_proto_enum_name, Some(variant.proto_name));
            self.append_field_attributes(&fq_proto_enum_name, variant.proto_name);
            self.push_indent();
            self.buf.push_str(&variant.generated_variant_name);
            self.buf.push_str(" = ");
            self.buf.push_str(&variant.proto_number.to_string());
            self.buf.push_str(",\n");

            self.path.pop();
        }

        self.path.pop();
        self.depth -= 1;

        self.push_indent();
        self.buf.push_str("}\n");

        self.push_indent();
        self.buf.push_str("impl ");
        self.buf.push_str(&enum_name);
        self.buf.push_str(" {\n");
        self.depth += 1;
        self.path.push(2);

        self.push_indent();
        self.buf.push_str(
            "/// String value of the enum field names used in the ProtoBuf definition.\n",
        );
        self.push_indent();
        self.buf.push_str("///\n");
        self.push_indent();
        self.buf.push_str(
            "/// The values are not transformed in any way and thus are considered stable\n",
        );
        self.push_indent();
        self.buf.push_str(
            "/// (if the ProtoBuf definition does not change) and safe for programmatic use.\n",
        );
        self.push_indent();
        self.buf
            .push_str("pub fn as_str_name(&self) -> &'static str {\n");
        self.depth += 1;

        self.push_indent();
        self.buf.push_str("match self {\n");
        self.depth += 1;

        for variant in variant_mappings.iter() {
            self.push_indent();
            self.buf.push_str("Self::");
            self.buf.push_str(&variant.generated_variant_name);
            self.buf.push_str(" => \"");
            self.buf.push_str(variant.proto_name);
            self.buf.push_str("\",\n");
        }

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n"); // End of match

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n"); // End of as_str_name()

        self.push_indent();
        self.buf
            .push_str("/// Creates an enum from field names used in the ProtoBuf definition.\n");

        self.push_indent();
        self.buf
            .push_str("pub fn from_str_name(value: &str) -> Option<Self> {\n");
        self.depth += 1;

        self.push_indent();
        self.buf.push_str("match value {\n");
        self.depth += 1;

        for variant in variant_mappings.iter() {
            self.push_indent();
            self.buf.push('\"');
            self.buf.push_str(variant.proto_name);
            self.buf.push_str("\" => Some(Self::");
            self.buf.push_str(&variant.generated_variant_name);
            self.buf.push_str("),\n");
        }
        self.push_indent();
        self.buf.push_str("_ => None,\n");

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n"); // End of match

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n"); // End of from_str_name()

        self.path.pop();
        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n"); // End of impl
    }

    fn push_service(&mut self, service: ServiceDescriptorProto) {
        let name = service.name().to_owned();
        debug!("  service: {:?}", name);

        let comments = self
            .location()
            .map(Comments::from_location)
            .unwrap_or_default();

        self.path.push(2);
        let methods = service
            .method
            .into_iter()
            .enumerate()
            .map(|(idx, mut method)| {
                debug!("  method: {:?}", method.name());

                self.path.push(idx as i32);
                let comments = self
                    .location()
                    .map(Comments::from_location)
                    .unwrap_or_default();
                self.path.pop();

                let name = method.name.take().unwrap();
                let input_proto_type = method.input_type.take().unwrap();
                let output_proto_type = method.output_type.take().unwrap();
                let input_type = self.resolve_ident(&input_proto_type);
                let output_type = self.resolve_ident(&output_proto_type);
                let client_streaming = method.client_streaming();
                let server_streaming = method.server_streaming();

                Method {
                    name: to_snake(&name),
                    proto_name: name,
                    comments,
                    input_type,
                    output_type,
                    input_proto_type,
                    output_proto_type,
                    options: method.options.unwrap_or_default(),
                    client_streaming,
                    server_streaming,
                }
            })
            .collect();
        self.path.pop();

        let service = Service {
            name: to_upper_camel(&name),
            proto_name: name,
            package: self.package.clone(),
            comments,
            methods,
            options: service.options.unwrap_or_default(),
        };

        if let Some(service_generator) = self.context.service_generator_mut() {
            service_generator.generate(service, self.buf)
        }
    }

    fn push_indent(&mut self) {
        push_indent(self.buf, self.depth);
    }

    fn push_mod(&mut self, module: &str) {
        self.push_indent();
        self.buf.push_str("/// Nested message and enum types in `");
        self.buf.push_str(module);
        self.buf.push_str("`.\n");

        self.push_indent();
        self.buf.push_str("pub mod ");
        self.buf.push_str(&to_snake(module));
        self.buf.push_str(" {\n");

        self.type_path.push(module.into());

        self.depth += 1;
    }

    fn pop_mod(&mut self) {
        self.depth -= 1;

        self.type_path.pop();

        self.push_indent();
        self.buf.push_str("}\n");
    }

    fn resolve_type(&self, field: &FieldDescriptorProto, fq_message_name: &str) -> String {
        match field.r#type() {
            Type::Float => String::from("f32"),
            Type::Double => String::from("f64"),
            Type::Uint32 | Type::Fixed32 => String::from("u32"),
            Type::Uint64 | Type::Fixed64 => String::from("u64"),
            Type::Int32 | Type::Sfixed32 | Type::Sint32 | Type::Enum => String::from("i32"),
            Type::Int64 | Type::Sfixed64 | Type::Sint64 => String::from("i64"),
            Type::Bool => String::from("bool"),
            Type::String => String::from("alloc::string::String"),
            Type::Bytes => self
                .context
                .bytes_type(fq_message_name, field.name())
                .rust_type()
                .to_owned(),
            Type::Group | Type::Message => self.resolve_ident(field.type_name()),
        }
    }

    fn resolve_ident(&self, pb_ident: &str) -> String {
        // protoc should always give fully qualified identifiers.
        assert_eq!(".", &pb_ident[..1]);

        if let Some(proto_ident) = self.context.resolve_extern_ident(pb_ident) {
            return proto_ident;
        }

        let mut local_path = self
            .package
            .split('.')
            .chain(self.type_path.iter().map(String::as_str))
            .peekable();

        // If no package is specified the start of the package name will be '.'
        // and split will return an empty string ("") which breaks resolution
        // The fix to this is to ignore the first item if it is empty.
        if local_path.peek().is_some_and(|s| s.is_empty()) {
            local_path.next();
        }

        let mut ident_path = pb_ident[1..].split('.');
        let ident_type = ident_path.next_back().unwrap();
        let mut ident_path = ident_path.peekable();

        // Skip path elements in common.
        while local_path.peek().is_some() && local_path.peek() == ident_path.peek() {
            local_path.next();
            ident_path.next();
        }

        local_path
            .map(|_| "super".to_string())
            .chain(ident_path.map(to_snake))
            .chain(iter::once(to_upper_camel(ident_type)))
            .join("::")
    }

    fn optional(&self, field: &FieldDescriptorProto) -> bool {
        if field.proto3_optional.unwrap_or(false) {
            return true;
        }

        if field.label() != Label::Optional {
            return false;
        }

        match field.r#type() {
            Type::Message => true,
            _ => self.syntax == Syntax::Proto2,
        }
    }

    /// Returns the fully-qualified name, starting with a dot
    fn fq_name(&self, message_name: &str) -> String {
        format!(
            "{}{}{}{}.{}",
            if self.package.is_empty() { "" } else { "." },
            self.package.trim_matches('.'),
            if self.type_path.is_empty() { "" } else { "." },
            self.type_path.join("."),
            message_name,
        )
    }
}

struct EnumVariantMapping<'a> {
    path_idx: usize,
    proto_name: &'a str,
    proto_number: i32,
    generated_variant_name: String,
}

fn build_enum_value_mappings<'a>(
    generated_enum_name: &str,
    do_strip_enum_prefix: bool,
    enum_values: &'a [EnumValueDescriptorProto],
) -> Vec<EnumVariantMapping<'a>> {
    let mut numbers = HashSet::new();
    let mut generated_names = HashMap::new();
    let mut mappings = Vec::new();

    for (idx, value) in enum_values.iter().enumerate() {
        // Skip duplicate enum values. Protobuf allows this when the
        // 'allow_alias' option is set.
        if !numbers.insert(value.number()) {
            continue;
        }

        let mut generated_variant_name = to_upper_camel(value.name());
        if do_strip_enum_prefix {
            generated_variant_name =
                strip_enum_prefix(generated_enum_name, &generated_variant_name);
        }

        if let Some(old_v) = generated_names.insert(generated_variant_name.to_owned(), value.name())
        {
            panic!(
                "Generated enum variant names overlap: `{}` variant name to be used both by `{}` and `{}` ProtoBuf enum values",
                generated_variant_name,
                old_v,
                value.name()
            );
        }

        mappings.push(EnumVariantMapping {
            path_idx: idx,
            proto_name: value.name(),
            proto_number: value.number(),
            generated_variant_name,
        })
    }
    mappings
}
