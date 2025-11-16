use std::any::TypeId;


pub trait PipelineGroup: 'static {
    fn name(&self) -> &'static str where Self: 'static { std::any::type_name::<Self>() }
    fn instance() -> &'static dyn PipelineGroup where Self: Sized;
    fn type_id(&self) -> TypeId where Self: 'static { TypeId::of::<Self>() }
    fn before(&self) -> &'static [TypeId] { &[] }
    fn after(&self) -> &'static [TypeId] { &[] }
    fn parent(&self) -> Option<TypeId> { None }
}

pub trait PipelineStage: 'static {
    fn run(&self);
    fn name(&self) -> &'static str { std::any::type_name::<Self>() }
    fn type_id(&self) -> TypeId where Self: 'static { TypeId::of::<Self>() }
    fn before(&self) -> &'static [TypeId] { &[] }
    fn after(&self) -> &'static [TypeId] { &[] }
    fn reads(&self) -> &'static [TypeId] { &[] }
    fn writes(&self) -> &'static [TypeId] { &[] }
}