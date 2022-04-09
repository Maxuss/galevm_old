use crate::features::StdFeature;
use crate::visit::Visitor;

macro_rules! exports {
    ($visitor:ident => $from:literal[$($id:ident),* $(,)*]) => {
        $(
        $visitor.import($from.to_string(), stringify!($id).to_string());
        )*
    };
}

#[doc(hidden)]
pub fn __prelude_features<V>(visitor: &mut V) where V: Visitor {
    visitor.add_std_feature(StdFeature::Core);
    visitor.add_std_feature(StdFeature::IO);
    visitor.add_std_feature(StdFeature::Strings);
    visitor.add_std_feature(StdFeature::Math);

    exports!(visitor => "std::io"[print, println, debug, fmt]);
    exports!(visitor => "std::math"[min, max, pow, cmp]);
    exports!(visitor => "std::str"[stringify]);
    exports!(visitor => "std"[exit, panic, sleep]);
}