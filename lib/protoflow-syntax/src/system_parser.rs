// This is free and unencumbered software released into the public domain.

use crate::prelude::{vec, BTreeSet, String};
use displaydoc::Display;
use protoflow_blocks::BLOCKS;
use sysml_model::QualifiedName;
use sysml_parser::{ParsedBlock, ParsedMember, ParsedPackage};

#[derive(Debug, Default)]
pub struct SystemParser {
    pub(crate) imported_names: BTreeSet<QualifiedName>,
}

impl SystemParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(&mut self, _input: &str) -> Result<(), ParseError> {
        todo!()
    }

    pub fn check(&mut self, package: ParsedPackage) -> Result<(), AnalyzeError> {
        self.check_usage(&ParsedMember::Package(package))
    }

    pub fn check_usage(&mut self, member: &ParsedMember) -> Result<(), AnalyzeError> {
        match member {
            ParsedMember::Import(import) => match import.imported_name.to_tuple3() {
                (Some("Protoflow"), Some("*") | Some("**"), None) => {
                    for block_name in BLOCKS.iter() {
                        self.imported_names.insert(QualifiedName::new(vec![
                            "Protoflow".into(),
                            (*block_name).into(),
                        ]));
                    }
                }
                (Some("Protoflow"), Some(unqualified_name), None) => {
                    if !BLOCKS
                        .iter()
                        .any(|block_name| *block_name == unqualified_name)
                    {
                        return Err(AnalyzeError::InvalidImport(import.imported_name.clone()));
                    }
                    self.imported_names.insert(import.imported_name.clone());
                }
                _ => {
                    return Err(AnalyzeError::InvalidImport(import.imported_name.clone()));
                }
            },
            ParsedMember::Package(package) => {
                for member in package.members() {
                    self.check_usage(&member)?;
                }
            }
            ParsedMember::BlockUsage(block) => {
                if let Some(definition_name) = &block.definition {
                    if !self.imported_names.contains(&definition_name) {
                        return Err(AnalyzeError::UnknownName(definition_name.clone()));
                    }
                }
                let _ = self.check_block(block)?;
            }
            ParsedMember::AttributeUsage(attribute) => {
                if let Some(definition_name) = &attribute.definition {
                    if !self.imported_names.contains(&definition_name) {
                        return Err(AnalyzeError::UnknownName(definition_name.clone()));
                    }
                }
            }
            ParsedMember::PortUsage(_port) => {
                unreachable!("PortUsage")
            }
        };
        Ok(())
    }

    pub fn check_block(&mut self, _member: &ParsedBlock) -> Result<(), AnalyzeError> {
        Ok(()) // TODO
    }
}

#[derive(Debug, Display)]
pub enum ParseError {
    /// Other error: `{0}`.
    Other(String),
}

#[derive(Debug, Display)]
pub enum AnalyzeError {
    /// Invalid import: `{0}`.
    InvalidImport(QualifiedName),
    /// Unknown name: `{0}`.
    UnknownName(QualifiedName),
    /// Other error: `{0}`.
    Other(String),
}