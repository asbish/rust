use rustc_ast::ast::Attribute;
use rustc_hir as hir;
use rustc_hir::def_id::DefId;
use rustc_hir::itemlikevisit::ItemLikeVisitor;
use rustc_hir::ItemKind;
use rustc_middle::ty::layout::HasDataLayout;
use rustc_middle::ty::layout::HasParamEnv;
use rustc_middle::ty::layout::HasTyCtxt;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::layout::TargetDataLayout;
use rustc_middle::ty::layout::TyAndLayout;
use rustc_middle::ty::ParamEnv;
use rustc_middle::ty::Ty;
use rustc_middle::ty::TyCtxt;
use rustc_span::symbol::sym;

pub fn test_layout(tcx: TyCtxt<'_>) {
    if tcx.features().rustc_attrs {
        // if the `rustc_attrs` feature is not enabled, don't bother testing layout
        tcx.hir().krate().visit_all_item_likes(&mut LayoutTest { tcx });
    }
}

struct LayoutTest<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl ItemLikeVisitor<'tcx> for LayoutTest<'tcx> {
    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        let item_def_id = self.tcx.hir().local_def_id(item.hir_id);

        match item.kind {
            ItemKind::TyAlias(..)
            | ItemKind::Enum(..)
            | ItemKind::Struct(..)
            | ItemKind::Union(..) => {
                for attr in self.tcx.get_attrs(item_def_id).iter() {
                    if attr.check_name(sym::rustc_layout) {
                        self.dump_layout_of(item_def_id, item, attr);
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_trait_item(&mut self, _: &'tcx hir::TraitItem<'tcx>) {}
    fn visit_impl_item(&mut self, _: &'tcx hir::ImplItem<'tcx>) {}
}

impl LayoutTest<'tcx> {
    fn dump_layout_of(&self, item_def_id: DefId, item: &hir::Item<'tcx>, attr: &Attribute) {
        let tcx = self.tcx;
        let param_env = self.tcx.param_env(item_def_id);
        let ty = self.tcx.type_of(item_def_id);
        match self.tcx.layout_of(param_env.and(ty)) {
            Ok(ty_layout) => {
                // Check out the `#[rustc_layout(..)]` attribute to tell what to dump.
                // The `..` are the names of fields to dump.
                let meta_items = attr.meta_item_list().unwrap_or_default();
                for meta_item in meta_items {
                    match meta_item.name_or_empty() {
                        sym::abi => {
                            self.tcx.sess.span_err(item.span, &format!("abi: {:?}", ty_layout.abi));
                        }

                        sym::align => {
                            self.tcx
                                .sess
                                .span_err(item.span, &format!("align: {:?}", ty_layout.align));
                        }

                        sym::size => {
                            self.tcx
                                .sess
                                .span_err(item.span, &format!("size: {:?}", ty_layout.size));
                        }

                        sym::homogeneous_aggregate => {
                            self.tcx.sess.span_err(
                                item.span,
                                &format!(
                                    "homogeneous_aggregate: {:?}",
                                    ty_layout
                                        .homogeneous_aggregate(&UnwrapLayoutCx { tcx, param_env }),
                                ),
                            );
                        }

                        sym::debug => {
                            self.tcx.sess.span_err(
                                item.span,
                                &format!("layout debugging: {:#?}", *ty_layout),
                            );
                        }

                        name => {
                            self.tcx.sess.span_err(
                                meta_item.span(),
                                &format!("unrecognized field name `{}`", name),
                            );
                        }
                    }
                }
            }

            Err(layout_error) => {
                self.tcx.sess.span_err(item.span, &format!("layout error: {:?}", layout_error));
            }
        }
    }
}

struct UnwrapLayoutCx<'tcx> {
    tcx: TyCtxt<'tcx>,
    param_env: ParamEnv<'tcx>,
}

impl LayoutOf for UnwrapLayoutCx<'tcx> {
    type Ty = Ty<'tcx>;
    type TyAndLayout = TyAndLayout<'tcx>;

    fn layout_of(&self, ty: Ty<'tcx>) -> Self::TyAndLayout {
        self.tcx.layout_of(self.param_env.and(ty)).unwrap()
    }
}

impl HasTyCtxt<'tcx> for UnwrapLayoutCx<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
}

impl HasParamEnv<'tcx> for UnwrapLayoutCx<'tcx> {
    fn param_env(&self) -> ParamEnv<'tcx> {
        self.param_env
    }
}

impl HasDataLayout for UnwrapLayoutCx<'tcx> {
    fn data_layout(&self) -> &TargetDataLayout {
        self.tcx.data_layout()
    }
}
