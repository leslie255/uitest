use std::marker::PhantomData;

use crate::{
    impl_view_list,
    view::{View, ViewList},
};

macro_rules! define_ViewListN {
    ($name:ident, $($fields:ident : $ty_params:ident),* $(,)?) => {
        pub struct $name<UiState, $($ty_params),*> {
            $(pub $fields : $ty_params,)*
            _marker: PhantomData<UiState>,
        }
        impl<UiState, $($ty_params),*> $name<UiState, $($ty_params),*> {
            #[allow(clippy::too_many_arguments)]
            pub const fn new($($fields : $ty_params),*) -> Self {
                Self {
                    $($fields,)*
                    _marker: PhantomData,
                }
            }
        }
        impl<'cx, UiState: 'cx, $($ty_params),*> ViewList<'cx> for $name<UiState, $($ty_params),*>
        where
            $($ty_params : View<'cx, UiState> + 'cx),*
        {
            type UiState = UiState;
            impl_view_list! { 'cx, $($fields),* }
        }
    };
}

impl<UiState> Default for ViewList0<UiState, > {
    fn default() -> Self {
        Self::new()
    }
}

define_ViewListN! { ViewList0, }
define_ViewListN! { ViewList1, view0: View0 }
define_ViewListN! { ViewList2, view0: View0, view1: View1 }
define_ViewListN! { ViewList3, view0: View0, view1: View1, view2: View2 }
define_ViewListN! { ViewList4, view0: View0, view1: View1, view2: View2, view3: View3 }
define_ViewListN! { ViewList5, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4 }
define_ViewListN! { ViewList6, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5 }
define_ViewListN! { ViewList7, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6 }
define_ViewListN! { ViewList8, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7 }
define_ViewListN! { ViewList9, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8 }
define_ViewListN! { ViewList10, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9 }
define_ViewListN! { ViewList11, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10 }
define_ViewListN! { ViewList12, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11 }
define_ViewListN! { ViewList13, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12 }
define_ViewListN! { ViewList14, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12, view13: View13 }
define_ViewListN! { ViewList15, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12, view13: View13, view14: View14 }
define_ViewListN! { ViewList16, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12, view13: View13, view14: View14, view15: View15 }

macro_rules! define_append {
    ($viewlist:ident -> $viewlist_next:ident, $($xs:ident : $ts:ident),* .. $y:ident : $u:ident $(,)?) => {
        impl<UiState, $($ts),*> $viewlist<UiState, $($ts),*> {
            pub fn append<$u>(self, $y: $u) -> $viewlist_next<UiState, $($ts,)* $u> {
                $viewlist_next::new($(self.$xs,)* $y)
            }
        }
    };
}

define_append! { ViewList0 -> ViewList1, .. view1: View1 }
define_append! { ViewList1 -> ViewList2, view0: View0 .. view1: View1 }
define_append! { ViewList2 -> ViewList3, view0: View0, view1: View1 .. view2: View2 }
define_append! { ViewList3 -> ViewList4, view0: View0, view1: View1, view2: View2 .. view3: View3 }
define_append! { ViewList4 -> ViewList5, view0: View0, view1: View1, view2: View2, view3: View3 .. view4: View4 }
define_append! { ViewList5 -> ViewList6, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4 .. view5: View5 }
define_append! { ViewList6 -> ViewList7, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5 .. view6: View6 }
define_append! { ViewList7 -> ViewList8, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6 .. view7: View7 }
define_append! { ViewList8 -> ViewList9, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7 .. view8: View8 }
define_append! { ViewList9 -> ViewList10, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8 .. view9: View9 }
define_append! { ViewList10 -> ViewList11, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9 .. view10: View10 }
define_append! { ViewList11 -> ViewList12, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10 .. view11: View11 }
define_append! { ViewList12 -> ViewList13, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11 .. view12: View12 }
define_append! { ViewList13 -> ViewList14, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12 .. view13: View13 }
define_append! { ViewList14 -> ViewList15, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12, view13: View13 .. view14: View14 }
define_append! { ViewList15 -> ViewList16, view0: View0, view1: View1, view2: View2, view3: View3, view4: View4, view5: View5, view6: View6, view7: View7, view8: View8, view9: View9, view10: View10, view11: View11, view12: View12, view13: View13, view14: View14 .. view15: View15 }
