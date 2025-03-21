use std::borrow::Cow;

use prost_types::{
    FieldDescriptorProto,
    field_descriptor_proto::{Label, Type},
};

use crate::extern_paths::ExternPaths;
use crate::message_graph::MessageGraph;
use crate::{BytesType, Config, MapType, ServiceGenerator};

/// The context providing all the global information needed to generate code.
/// It also provides a more disciplined access to Config
/// and its mutable instance of ServiceGenerator.
///
/// A `Context` is built once in the generation process and is reused by
/// `CodeGenerator` instances created to generate code for each input file.
pub struct Context<'a> {
    config: &'a mut Config,
    message_graph: MessageGraph,
    extern_paths: ExternPaths,
}

impl<'a> Context<'a> {
    pub fn new(
        config: &'a mut Config,
        message_graph: MessageGraph,
        extern_paths: ExternPaths,
    ) -> Self {
        Self {
            config,
            message_graph,
            extern_paths,
        }
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    pub fn service_generator_mut(&mut self) -> Option<&mut (dyn ServiceGenerator + 'static)> {
        self.config.service_generator.as_deref_mut()
    }

    pub fn resolve_extern_ident(&self, pb_ident: &str) -> Option<String> {
        self.extern_paths.resolve_ident(pb_ident)
    }

    /// Returns an iterator over the additional attributes configured
    /// for the named type.
    pub fn type_attributes(&self, fq_type_name: &str) -> impl Iterator<Item = &str> {
        self.config
            .type_attributes
            .get(fq_type_name)
            .map(|s| s.as_str())
    }

    /// Returns an iterator over the additional attributes configured
    /// for the named message.
    pub fn message_attributes(&self, fq_message_name: &str) -> impl Iterator<Item = &str> {
        self.config
            .message_attributes
            .get(fq_message_name)
            .map(|s| s.as_str())
    }

    /// Returns an iterator over the additional attributes configured
    /// for the named enum.
    pub fn enum_attributes(&self, fq_enum_name: &str) -> impl Iterator<Item = &str> {
        self.config
            .enum_attributes
            .get(fq_enum_name)
            .map(|s| s.as_str())
    }

    /// Returns an iterator over the additional attributes configured
    /// for the named message field.
    pub fn field_attributes(
        &self,
        fq_message_name: &str,
        field_name: &str,
    ) -> impl Iterator<Item = &str> {
        self.config
            .field_attributes
            .get_field(fq_message_name, field_name)
            .map(|s| s.as_str())
    }

    /// Returns the bytes type configured for the named message field.
    pub(crate) fn bytes_type(&self, fq_message_name: &str, field_name: &str) -> BytesType {
        self.config
            .bytes_type
            .get_first_field(fq_message_name, field_name)
            .copied()
            .unwrap_or_default()
    }

    /// Returns the map type configured for the named message field.
    pub(crate) fn map_type(&self, fq_message_name: &str, field_name: &str) -> MapType {
        self.config
            .map_type
            .get_first_field(fq_message_name, field_name)
            .copied()
            .unwrap_or_default()
    }

    /// Returns whether the Rust type for this message field needs to be `Box<_>`.
    ///
    /// This can be explicitly configured with `Config::boxed`, or necessary
    /// to prevent an infinitely sized type definition in case when the type of
    /// a non-repeated message field transitively contains the message itself.
    pub fn should_box_message_field(
        &self,
        fq_message_name: &str,
        field: &FieldDescriptorProto,
    ) -> bool {
        self.should_box_impl(fq_message_name, None, field)
    }

    /// Returns whether the Rust type for this field in the oneof needs to be `Box<_>`.
    ///
    /// This can be explicitly configured with `Config::boxed`, or necessary
    /// to prevent an infinitely sized type definition in case when the type of
    /// a non-repeated message field transitively contains the message itself.
    pub fn should_box_oneof_field(
        &self,
        fq_message_name: &str,
        oneof_name: &str,
        field: &FieldDescriptorProto,
    ) -> bool {
        self.should_box_impl(fq_message_name, Some(oneof_name), field)
    }

    fn should_box_impl(
        &self,
        fq_message_name: &str,
        oneof: Option<&str>,
        field: &FieldDescriptorProto,
    ) -> bool {
        if field.label() == Label::Repeated {
            // Repeated field are stored in Vec, therefore it is already heap allocated
            return false;
        }
        let fd_type = field.r#type();
        if (fd_type == Type::Message || fd_type == Type::Group)
            && self
                .message_graph
                .is_nested(field.type_name(), fq_message_name)
        {
            return true;
        }
        let config_path = match oneof {
            None => Cow::Borrowed(fq_message_name),
            Some(oneof_name) => Cow::Owned(format!("{fq_message_name}.{oneof_name}")),
        };
        if self
            .config
            .boxed
            .get_first_field(&config_path, field.name())
            .is_some()
        {
            return true;
        }
        false
    }

    pub fn should_disable_comments(&self, fq_message_name: &str, field_name: Option<&str>) -> bool {
        if let Some(field_name) = field_name {
            self.config
                .disable_comments
                .get_first_field(fq_message_name, field_name)
                .is_some()
        } else {
            self.config
                .disable_comments
                .get(fq_message_name)
                .next()
                .is_some()
        }
    }
}
