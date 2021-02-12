use crate::form::TESForm;

#[repr(C)]
pub(crate) struct TESObjectREFR {
    pub(crate) form: TESForm,
    unknown20: u64,
    unknown28: u64,
    unknown30: u64,
    unknown38: u64,
    pub(crate) base_form: *const TESForm,
}
