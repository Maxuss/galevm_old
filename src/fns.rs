use std::cmp::max;
use crate::tks::{Literal, TokenChain};
use crate::var::ContainingScope;
use crate::visit::{Scope, Visitor};
use crate::vm::Transmute;
use rand::RngCore;
use std::fmt::Debug;
use std::io::Cursor;
use std::sync::Mutex;
use anyhow::bail;
use lazy_static::lazy_static;

pub type Parameters = Vec<Literal>;
pub type DynExecutable = dyn Fn(Parameters) -> Literal + Sync + Send;

lazy_static! {
    pub static ref EXTERN_FNS: Mutex<Vec<Box<DynExecutable>>> = Mutex::new(Vec::new());
}

#[inline]
pub fn import_globals<V>(scope: &mut ContainingScope, visitor: &mut V) where V: Visitor {
    for (from, imports) in visitor.get_scope("global".to_string()).lock().unwrap().imports() {
        for import in imports {
            scope.import(&from, &import);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StaticFnType {
    Standard(StaticFn),
    Extern(ExternFn)
}

impl StaticFnType {
    pub fn call<V>  (&self, params: Parameters, visitor: Option<&mut V>) -> Literal where V: Visitor {
        match self {
            StaticFnType::Standard(std) => std.call(params, visitor.unwrap()),
            StaticFnType::Extern(ext) => ext.call(params)
        }
    }
}

impl Transmute for StaticFnType {
    fn size(&mut self) -> usize {
        1 + match self {
            StaticFnType::Standard(std) => std.size(),
            StaticFnType::Extern(ext) => ext.size()
        }
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            StaticFnType::Standard(std) => {
                0x01u8.write(buf)?;
                std.write(buf)
            }
            StaticFnType::Extern(ext) => {
                0x01u8.write(buf)?;
                ext.write(buf)
            }
        }
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self> where Self: Sized {
        match u8::read(buf)? {
            0x01 => Ok(StaticFnType::Standard(StaticFn::read(buf)?)),
            0x02 => Ok(StaticFnType::Extern(ExternFn::read(buf)?)),
            _ => bail!("Invalid static fn id provided!")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticFn {
    out_ty: String,
    param_names: Vec<String>,
    chain: TokenChain,
}

impl Transmute for StaticFn {
    fn size(&mut self) -> usize {
        self.out_ty.size() + self.param_names.len() + 4 + self.chain.len()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.out_ty.write(buf)?;
        self.param_names.write(buf)?;
        self.chain.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(StaticFn::new(
            String::read(buf)?,
            Vec::read(buf)?,
            TokenChain::read(buf)?,
        ))
    }
}

impl StaticFn {
    pub fn new(out_ty: String, param_names: Vec<String>, chain: TokenChain) -> Self {
        Self {
            out_ty,
            param_names,
            chain,
        }
    }

    pub fn call<V>(&self, params: Parameters, visitor: &mut V) -> Literal
    where
        V: Visitor,
    {
        if !self.param_names.contains(&"varargs".to_string()) && params.len() != self.param_names.len() {
            panic!(
                "Invalid amount of arguments supplied! Expected {} arg(s)!",
                self.param_names.len()
            );
        };

        // preparing scope and injecting arguments
        let mut scope = ContainingScope::new();
        for pid in 0..self.param_names.len() {
            scope.add_const(self.param_names[pid].as_str(), params[pid].to_owned());
        }

        import_globals(&mut scope, visitor);

        // creating scope
        let cached = visitor.scope_name();
        let name = format!("static_fn_0x{:2x}", rand::thread_rng().next_u64());
        visitor.push_scope_level(Scope::InstanceFunction);
        visitor.push_scope(name.clone(), scope);

        visitor.move_scope(name.clone());

        // processing tokens
        visitor.load_chain(&mut self.chain.clone());
        visitor.process();
        let output = visitor.pop_stack();
        if self.out_ty != "unknown" && !output.type_str(&self.out_ty) {
            panic!(
                "Invalid output provided! Expected output of type {:?}",
                self.out_ty
            )
        };

        // changing scopes back
        visitor.move_scope(cached);
        visitor.drop_scope(name);
        visitor.pop_scope_level();

        output
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFn {
    out_ty: String,
    param_names: Vec<String>,
    handler: usize
}

impl ExternFn {
    pub fn new(out_ty: String, param_names: Vec<String>, handler: usize) -> Self {
        Self {
            out_ty,
            param_names,
            handler
        }
    }

    pub fn call(&self, params: Parameters) -> Literal
    {
        if !self.param_names.contains(&"varargs".to_string()) && params.len() != self.param_names.len() {
            panic!(
                "Invalid amount of arguments supplied! Expected {} arg(s)!",
                self.param_names.len()
            );
        };

        let fun = &EXTERN_FNS.lock().unwrap()[max(0, self.handler - 1)];
        fun.call((params, ))
    }
}

impl Transmute for ExternFn {
    fn size(&mut self) -> usize {
        self.out_ty.size() + self.param_names.size() + (self.handler as u64).size()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.out_ty.write(buf)?;
        self.param_names.write(buf)?;
        (self.handler as u64).write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self> where Self: Sized {
        let out_ty = String::read(buf)?;
        let param_names = Vec::<String>::read(buf)?;
        let handler = u64::read(buf)?;
        Ok(Self {
            out_ty,
            param_names,
            handler: handler as usize
        })

    }
}

#[macro_export]
macro_rules! extern_fns {
    ($vm:ident {
        $(
            extern fn $name:ident ($($param:ident),* $(,)*) -> $out_ty:ident;
        )*
    }) => {
        {
            let mut __extfns = &mut $crate::fns::EXTERN_FNS.lock().unwrap();
            #[allow(unused_imports)]
            use $crate::visit::ScopeProvider;
            $(
                __extfns.push(Box::new($name));
                $vm.add_extern_fn(stringify!($name).to_string(), stringify!($out_ty).to_string(), vec![$(stringify!($param).to_string()),*], __extfns.len());
            )*
            drop(__extfns);
        }
    };

    ($vm:ident {
        $(
            scope $scope:literal {
                $(
                    extern fn $name:ident ($($param:ident),* $(,)*) -> $out_ty:ident;
                )*
            }
        )*
    }) => {
        {
            let mut __extfns = &mut $crate::fns::EXTERN_FNS.lock().unwrap();
            $(
                let mut scope = $crate::var::ContainingScope::new();
                $(
                    scope.export(stringify!($name));
                    __extfns.push(Box::new($name));
                    scope.add_extern_fn(stringify!($name), stringify!($out_ty).to_string(), vec![$(stringify!($param).to_string()),*], __extfns.len());
                )*
                $vm.push_scope($scope.to_string(), scope);
            )*
            drop(__extfns);
        }
    }
}

#[macro_export]
macro_rules! unwrap_args {
    ($params:ident => ($($lit:ident),* $(,)*)) => {
        {
            let mut vec = std::collections::VecDeque::from($params.to_owned());
            (
                    $(
                    match vec.pop_back().unwrap() {
                        $crate::tks::Literal::$lit(val) => val.to_owned(),
                        _ => panic!("Expected {} literal!", stringify!($lit))
                    }
                ),*
            )
        }
    };
}