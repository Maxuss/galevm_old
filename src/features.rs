use crate::stdlib::__core_feature;
use crate::stdlib::io::__io_feature;
use crate::stdlib::math::__math_feature;
use crate::stdlib::strs::__str_feature;
use crate::visit::Visitor;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum StdFeature {
    Core,
    IO,
    Math,
    Strings
}

impl StdFeature {
    pub fn include<V>(&self, visitor: &mut V) where V: Visitor {
        match *self {
            StdFeature::Core => __core_feature(visitor),
            StdFeature::IO => __io_feature(visitor),
            StdFeature::Math => __math_feature(visitor),
            StdFeature::Strings => __str_feature(visitor)
        }
    }
}