/*
 * SGW.exe Decompilation - 03_cegui_ui
 * Classes: 1
 * Stargate Worlds Client
 */


/* ========== CEGUI.c ========== */

/*
 * SGW.exe - CEGUI (183 functions)
 */

/* [VTable] CEGUI_VSGWUIManager___MemberFunctionSlot virtual function #1
   VTable: vtable_CEGUI_VSGWUIManager___MemberFunctionSlot at 0183fa24 */

void __fastcall CEGUI_VSGWUIManager___MemberFunctionSlot__vfunc_1(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x00570118. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(param_1 + 4))();
  return;
}


/* Also in vtable: CEGUI_WindowEventArgs (slot 0) */

undefined4 * __thiscall CEGUI_WindowEventArgs__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_0168ee18;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = CEGUI::EventArgs::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* Also in vtable: CEGUI_MouseEventArgs (slot 0) */

undefined4 * __thiscall CEGUI_MouseEventArgs__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_0168ee40;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = CEGUI::EventArgs::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* Also in vtable: CEGUI_VSGWUIManager___MemberFunctionSlot (slot 0) */

undefined4 * __thiscall CEGUI_VSGWUIManager___MemberFunctionSlot__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_0168ee98;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = CEGUI::SlotFunctorBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* Also in vtable: CEGUI_Exception (slot 0) */

undefined4 * __thiscall CEGUI_Exception__vfunc_0(void *this,byte param_1)

{
  FUN_011c2480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_UE3Renderer virtual function #24
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __thiscall CEGUI_UE3Renderer__vfunc_24(void *this,float *param_1)

{
  float fVar1;
  float fVar2;
  float fVar3;
  float fVar4;
  float fVar5;
  float fVar6;
  float fVar7;
  int iVar8;
  int *piVar9;
  undefined4 *puVar10;
  int iVar11;
  undefined *this_00;
  float unaff_ESI;
  float *pfVar12;
  float unaff_retaddr;
  float fVar13;
  float local_c;
  float local_8;
  float local_4;
  
  fVar13 = param_1[0xe];
  if (fVar13 != 0.0) {
    local_4 = param_1[2];
    local_8 = *param_1;
    local_c = param_1[3] - local_4;
    pfVar12 = (float *)(param_1[1] - local_8);
    if (0.0 < param_1[6]) {
      local_4 = local_c - local_c / (DAT_017f7ea0 - param_1[6]);
      local_c = local_c - local_4;
    }
    if (0.0 < param_1[4]) {
      local_8 = (float)pfVar12 - (float)pfVar12 / (DAT_017f7ea0 - param_1[4]);
      pfVar12 = (float *)((float)pfVar12 - local_8);
    }
    fVar1 = param_1[7];
    if ((fVar1 < DAT_017f7ea0) && (0.0 < fVar1)) {
      local_c = local_c / fVar1;
    }
    fVar1 = param_1[5];
    param_1 = pfVar12;
    if ((fVar1 < DAT_017f7ea0) && (0.0 < fVar1)) {
      param_1 = (float *)((float)pfVar12 / fVar1);
    }
    this_00 = FlashExternalWindowModule__unknown_00569230();
    FUN_005680b0(this_00,fVar13,local_4,local_8,local_c,param_1);
    return;
  }
  iVar8 = (**(code **)(*(int *)param_1[0xc] + 0x28))();
  if (iVar8 != 0) {
    piVar9 = (int *)(**(code **)(*(int *)param_1[0xc] + 0x28))();
    fVar13 = param_1[1];
    fVar1 = *param_1;
    fVar2 = param_1[3];
    fVar3 = param_1[2];
    puVar10 = (undefined4 *)(**(code **)(*piVar9 + 300))(0);
    Aligned_operator_1(*(int *)((int)this + 0x68),param_1[2],*param_1,unaff_ESI,fVar2 - fVar3,
                       param_1[6],param_1[4],fVar13 - fVar1,unaff_retaddr,puVar10);
    return;
  }
  iVar8 = (**(code **)(*(int *)param_1[0xc] + 0x24))();
  if ((*(byte *)(iVar8 + 0x3c) & 4) == 0) {
    iVar8 = *(int *)((int)this + 0x3c);
  }
  else {
    iVar8 = 0;
  }
  fVar13 = param_1[5];
  fVar1 = param_1[4];
  fVar2 = param_1[7];
  fVar3 = param_1[6];
  fVar4 = param_1[1];
  fVar5 = *param_1;
  fVar6 = param_1[3];
  fVar7 = param_1[2];
  iVar11 = (**(code **)(*(int *)param_1[0xc] + 0x24))();
  FUN_005f83e0(*(float **)((int)this + 0x68),param_1[2],*param_1,fVar6 - fVar7,fVar4 - fVar5,
               param_1[6],param_1[4],fVar2 - fVar3,fVar13 - fVar1,(undefined8 *)(param_1 + 8),
               *(int *)(iVar11 + 0xb8),iVar8);
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #10
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

undefined4 CEGUI_UE3Renderer__vfunc_10(void)

{
  return 0;
}


/* [VTable] CEGUI_UE3Renderer virtual function #16
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

float10 __fastcall CEGUI_UE3Renderer__vfunc_16(int param_1)

{
  return (float10)*(float *)(param_1 + 0x4c) - (float10)*(float *)(param_1 + 0x48);
}


/* [VTable] CEGUI_UE3Renderer virtual function #17
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

float10 __fastcall CEGUI_UE3Renderer__vfunc_17(int param_1)

{
  return (float10)*(float *)(param_1 + 0x44) - (float10)*(float *)(param_1 + 0x40);
}


/* [VTable] CEGUI_UE3Renderer virtual function #18
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __thiscall CEGUI_UE3Renderer__vfunc_18(void *this,float *param_1)

{
  *param_1 = *(float *)((int)this + 0x4c) - *(float *)((int)this + 0x48);
  param_1[1] = *(float *)((int)this + 0x44) - *(float *)((int)this + 0x40);
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #19
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __thiscall CEGUI_UE3Renderer__vfunc_19(void *this,undefined8 *param_1)

{
  *param_1 = *(undefined8 *)((int)this + 0x40);
  param_1[1] = *(undefined8 *)((int)this + 0x48);
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #20
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

undefined4 CEGUI_UE3Renderer__vfunc_20(void)

{
  return 0x200;
}


/* [VTable] CEGUI_UE3Renderer virtual function #21
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

undefined4 CEGUI_UE3Renderer__vfunc_21(void)

{
  return 0x60;
}


/* [VTable] CEGUI_UE3Renderer virtual function #22
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

undefined4 CEGUI_UE3Renderer__vfunc_22(void)

{
  return 0x60;
}


/* [VTable] CEGUI_UE3Renderer virtual function #7
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __fastcall CEGUI_UE3Renderer__vfunc_7(int *param_1)

{
  int iVar1;
  int *piVar2;
  int iVar3;
  int *local_8;
  int local_4;
  
  local_4 = *(int *)param_1[0x15];
  local_8 = param_1 + 0x14;
  while( true ) {
    iVar3 = local_4;
    piVar2 = local_8;
    iVar1 = param_1[0x15];
    if ((local_8 == (int *)0x0) || (local_8 != param_1 + 0x14)) {
      _invalid_parameter_noinfo();
    }
    if (iVar3 == iVar1) break;
    if (piVar2 == (int *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (iVar3 == piVar2[1]) {
      _invalid_parameter_noinfo();
    }
    (**(code **)(*param_1 + 0x60))(iVar3 + 0xc);
    FUN_00940800((int *)&local_8);
  }
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #8
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __fastcall CEGUI_UE3Renderer__vfunc_8(int param_1)

{
  FUN_00940920(*(int *)(*(int *)(param_1 + 0x54) + 4));
  *(int *)(*(int *)(param_1 + 0x54) + 4) = *(int *)(param_1 + 0x54);
  *(undefined4 *)(param_1 + 0x58) = 0;
  *(undefined4 *)*(undefined4 *)(param_1 + 0x54) = *(undefined4 *)(param_1 + 0x54);
  *(int *)(*(int *)(param_1 + 0x54) + 8) = *(int *)(param_1 + 0x54);
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #14
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __fastcall CEGUI_UE3Renderer__vfunc_14(int *param_1)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = param_1[0x19];
  while (iVar1 != 0) {
    piVar2 = *(int **)param_1[0x18];
    if (piVar2 == (int *)param_1[0x18]) {
      _invalid_parameter_noinfo();
    }
    (**(code **)(*param_1 + 0x34))(piVar2[2]);
    iVar1 = param_1[0x19];
  }
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #12
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void * __fastcall CEGUI_UE3Renderer__vfunc_12(int param_1)

{
  int iVar1;
  void *pvVar2;
  undefined4 *puVar3;
  void *local_1c;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016c5a9b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_1c = (void *)scalable_malloc(0x10);
  if (local_1c == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  pvVar2 = (void *)FUN_00a28c50(local_1c,param_1);
  local_4 = 0xffffffff;
  iVar1 = *(int *)(param_1 + 0x60);
  local_1c = pvVar2;
  puVar3 = FUN_00d08360(iVar1,*(undefined4 *)(iVar1 + 4),&local_1c);
  FUN_00426770((void *)(param_1 + 0x5c),1);
  *(undefined4 **)(iVar1 + 4) = puVar3;
  *(undefined4 **)puVar3[1] = puVar3;
  ExceptionList = local_c;
  return pvVar2;
}


/* [VTable] CEGUI_UE3Renderer virtual function #11
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

int * __thiscall CEGUI_UE3Renderer__vfunc_11(void *this,undefined4 param_1,undefined4 param_2)

{
  int iVar1;
  undefined4 *puVar2;
  exception local_28 [12];
  int *local_1c;
  int *local_18;
  undefined1 *local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_016c5ab0;
  local_10 = ExceptionList;
  local_14 = &stack0xffffffcc;
  ExceptionList = &local_10;
  local_1c = (int *)scalable_malloc(0x10);
  if (local_1c == (int *)0x0) {
    FUN_004099f0(local_28);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_28,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_8 = 0;
  local_1c = (int *)FUN_00a28c50(local_1c,this);
  local_8 = 1;
  local_18 = local_1c;
  (**(code **)(*local_1c + 0x18))(param_1,param_2);
  local_8 = 0xffffffff;
  iVar1 = *(int *)((int)this + 0x60);
  puVar2 = FUN_00d08360(iVar1,*(undefined4 *)(iVar1 + 4),&local_1c);
  FUN_00426770((void *)((int)this + 0x5c),1);
  *(undefined4 **)(iVar1 + 4) = puVar2;
  *(undefined4 **)puVar2[1] = puVar2;
  ExceptionList = local_10;
  return local_18;
}


/* [VTable] CEGUI_UE3Renderer virtual function #13
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __thiscall CEGUI_UE3Renderer__vfunc_13(void *this,int *param_1)

{
  int *piVar1;
  
  piVar1 = param_1;
  if (param_1 != (int *)0x0) {
    FUN_00e6c2f0((void *)((int)this + 0x5c),(int *)&param_1);
    (**(code **)(*piVar1 + 0x20))(1);
  }
  return;
}


/* [VTable] CEGUI_UE3Renderer virtual function #15
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

undefined1 __fastcall CEGUI_UE3Renderer__vfunc_15(int param_1)

{
  return *(undefined1 *)(param_1 + 0x38);
}


/* [VTable] CEGUI_UE3Renderer virtual function #9
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __thiscall CEGUI_UE3Renderer__vfunc_9(void *this,undefined1 param_1)

{
  *(undefined1 *)((int)this + 0x38) = param_1;
  return;
}


/* Also in vtable: CEGUI_UE3Renderer (slot 0) */

int * __thiscall CEGUI_UE3Renderer__vfunc_0(void *this,byte param_1)

{
  FUN_009402e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CEGUI_UE3ScopedRenderToTextureState (slot 0) */

undefined4 * __thiscall CEGUI_UE3ScopedRenderToTextureState__vfunc_0(void *this,byte param_1)

{
  FUN_009404c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CEGUI_ResourceProvider (slot 0) */

undefined4 * __thiscall CEGUI_ResourceProvider__vfunc_0(void *this,byte param_1)

{
  FUN_00942b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_SGWResourceProvider virtual function #1
   VTable: vtable_CEGUI_SGWResourceProvider at 018faa90
   [String discovery] References: "SGWResourceProvider::load" */

void __thiscall
CEGUI_SGWResourceProvider__vfunc_1(void *this,void *param_1,undefined4 *param_2,void *param_3)

{
  undefined4 *puVar1;
  int iVar2;
  uint uVar3;
  void *pvVar4;
  undefined **ppuVar5;
  wchar_t local_f4 [2];
  void *local_f0;
  rsize_t local_ec;
  undefined4 local_e8;
  undefined1 auStack_e4 [4];
  wchar_t awStack_e0 [8];
  undefined4 uStack_d0;
  undefined4 uStack_cc;
  undefined1 auStack_c8 [4];
  wchar_t local_c4 [8];
  undefined4 local_b4;
  undefined4 local_b0;
  exception local_ac [12];
  undefined1 auStack_a0 [28];
  undefined1 auStack_84 [28];
  undefined1 auStack_68 [92];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016c5f7e;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  puVar1 = (undefined4 *)FUN_00943670(this,param_1,param_3);
  local_f0 = (void *)0x0;
  local_ec = 0;
  local_e8 = 0;
  local_4 = 1;
  if (puVar1[1] == 0) {
    ppuVar5 = &PTR_017f8e10;
  }
  else {
    ppuVar5 = (undefined **)*puVar1;
  }
  iVar2 = FUN_004870f0(&local_f0,ppuVar5,(int *)PTR_PTR_01dae7f8);
  if (iVar2 == 0) {
    local_b0 = 7;
    local_f4[0] = L'\0';
    local_f4[1] = L'\0';
    local_b4 = 0;
    std::char_traits<wchar_t>::assign(local_c4,local_f4);
    uVar3 = std::char_traits<wchar_t>::length(L".\\Src\\SGWUIResourceProvider.cpp");
    FUN_00438520(auStack_c8,L".\\Src\\SGWUIResourceProvider.cpp",uVar3);
    local_4._0_1_ = 2;
    if (puVar1[1] == 0) {
      ppuVar5 = &PTR_017f8e10;
    }
    else {
      ppuVar5 = (undefined **)*puVar1;
    }
    uStack_cc = 7;
    local_f4[0] = L'\0';
    local_f4[1] = L'\0';
    uStack_d0 = 0;
    std::char_traits<wchar_t>::assign(awStack_e0,local_f4);
    uVar3 = std::char_traits<wchar_t>::length((wchar_t *)ppuVar5);
    FUN_00438520(auStack_e4,(wchar_t *)ppuVar5,uVar3);
    local_4._0_1_ = 3;
    pvVar4 = FUN_0093d8c0(auStack_a0,L"SGWResourceProvider::load() - Problem reading \'",
                          (int)auStack_e4);
    local_4._0_1_ = 4;
    pvVar4 = FUN_0043a100(auStack_84,pvVar4,L"\'");
    local_4 = CONCAT31(local_4._1_3_,5);
    CEGUI_FileIOException(auStack_68,pvVar4,auStack_c8,0x78);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(auStack_68,(ThrowInfo *)&DAT_01c72770);
  }
  pvVar4 = (void *)scalable_malloc(local_ec);
  if (pvVar4 == (void *)0x0) {
    FUN_004099f0(local_ac);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_ac,(ThrowInfo *)&DAT_01d65cc8);
  }
  *param_2 = pvVar4;
  if (0 < (int)local_ec) {
    memmove_s(pvVar4,local_ec,local_f0,local_ec);
  }
  param_2[1] = local_ec;
  local_4 = 0xffffffff;
  FUN_0048ead0((int *)&local_f0);
  ExceptionList = pvStack_c;
  return;
}


/* Also in vtable: CEGUI_SGWResourceProvider (slot 0) */

undefined4 * __thiscall CEGUI_SGWResourceProvider__vfunc_0(void *this,byte param_1)

{
  FUN_00943a30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable:
   CEGUI_ScriptedWindow_CEGUI_VActivationEventArgs_U__UIEventHandler___FunctorCopySlot (slot 0) */

undefined4 * __thiscall
CEGUI_ScriptedWindow_CEGUI_VActivationEventArgs_U__UIEventHandler___FunctorCopySlot__vfunc_0
          (void *this,byte param_1)

{
  FUN_0094aab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable:
   CEGUI_ScriptedWindow_CEGUI_VHeaderSequenceEventArgs_U__UIEventHandler___FunctorCopySlot (slot 0)
    */

undefined4 * __thiscall
CEGUI_ScriptedWindow_CEGUI_VHeaderSequenceEventArgs_U__UIEventHandler___FunctorCopySlot__vfunc_0
          (void *this,byte param_1)

{
  FUN_0094adb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_Texture virtual function #1
   VTable: vtable_CEGUI_Texture at 0191ec0c */

void __fastcall CEGUI_Texture__vfunc_1(undefined4 *param_1)

{
                    /* WARNING: Could not recover jumptable at 0x00a28bb4. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)*param_1)();
  return;
}


/* [VTable] CEGUI_Texture virtual function #4
   VTable: vtable_CEGUI_Texture at 0191ec0c */

void __fastcall CEGUI_Texture__vfunc_4(int *param_1)

{
                    /* WARNING: Could not recover jumptable at 0x00a28be5. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*param_1 + 0xc))();
  return;
}


/* [VTable] CEGUI_Texture virtual function #8
   VTable: vtable_CEGUI_Texture at 0191ec0c */

undefined4 * __thiscall CEGUI_Texture__vfunc_8(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Texture::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_UE3Texture virtual function #9
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

undefined4 __fastcall CEGUI_UE3Texture__vfunc_9(int param_1)

{
  return *(undefined4 *)(param_1 + 8);
}


/* [VTable] CEGUI_UE3Texture virtual function #10
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

undefined4 __fastcall CEGUI_UE3Texture__vfunc_10(int param_1)

{
  return *(undefined4 *)(param_1 + 0xc);
}


/* [VTable] CEGUI_UE3Texture virtual function #3
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

undefined4 __fastcall CEGUI_UE3Texture__vfunc_3(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 8);
  if (iVar1 != 0) {
    return CONCAT22((short)((uint)iVar1 >> 0x10),*(undefined2 *)(iVar1 + 0xcc));
  }
  return 0x400;
}


/* Also in vtable: CEGUI_UE3Texture (slot 0) */

undefined4 __fastcall CEGUI_UE3Texture__vfunc_0(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 8);
  if (iVar1 != 0) {
    return CONCAT22((short)((uint)iVar1 >> 0x10),*(undefined2 *)(iVar1 + 200));
  }
  return 0x400;
}


/* [VTable] CEGUI_UE3Texture virtual function #1
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

undefined4 __fastcall CEGUI_UE3Texture__vfunc_1(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 8);
  if (iVar1 != 0) {
    return CONCAT22((short)((uint)iVar1 >> 0x10),*(undefined2 *)(iVar1 + 200));
  }
  return 0x400;
}


/* [VTable] CEGUI_UE3Texture virtual function #4
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

undefined4 __fastcall CEGUI_UE3Texture__vfunc_4(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 8);
  if (iVar1 != 0) {
    return CONCAT22((short)((uint)iVar1 >> 0x10),*(undefined2 *)(iVar1 + 0xcc));
  }
  return 0x400;
}


/* [VTable] CEGUI_UE3Texture virtual function #7
   VTable: vtable_CEGUI_UE3Texture at 0191ec34
   [String discovery] References: "Texture::PixelFormat::PF_RGBA" */

void __thiscall
CEGUI_UE3Texture__vfunc_7(void *this,void *param_1,uint param_2,int param_3,int param_4)

{
  int *piVar1;
  void *local_58;
  undefined4 local_54;
  undefined4 local_50;
  int local_4c [3];
  uint local_40;
  int local_3c;
  void **local_38;
  undefined4 *local_34;
  void *local_30;
  undefined4 local_2c;
  undefined4 local_28;
  undefined4 local_24;
  undefined4 local_20;
  undefined4 local_1c;
  undefined4 local_18;
  undefined4 local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016d4fbc;
  local_c = ExceptionList;
  local_58 = (void *)0x0;
  local_54 = 0;
  local_50 = 0;
  local_4 = 1;
  ExceptionList = &local_c;
  if (param_4 != 1) {
    ExceptionList = &local_c;
    FUN_00486000("Texture::PixelFormat::PF_RGBA == pixFormat",".\\Src\\UE3Texture.cpp",0x4e);
  }
  FUN_0041a950(&local_58,param_2 * param_3,4,8);
  memcpy(local_58,param_1,param_2 * param_3 * 4);
  local_30 = FUN_00fa1ec0(local_4c);
  local_4._0_1_ = 2;
  if (DAT_01ec69a0 == (undefined4 *)0x0) {
    DAT_01ec69a0 = FUN_00495690();
    FUN_00494830();
  }
  local_38 = &local_58;
  local_40 = param_2;
  local_3c = param_3;
  local_34 = DAT_01ec69a0;
  local_2c = 0;
  local_28 = 0x4000;
  local_24 = 0;
  local_20 = 0;
  local_1c = 0;
  local_18 = 1;
  local_14 = 0;
  local_10 = 0;
  local_4 = CONCAT31(local_4._1_3_,1);
  FUN_0041b420(local_4c);
  local_20 = 1;
  local_18 = 0;
  piVar1 = FUN_0099dea0(&local_40);
  *(int **)((int)this + 8) = piVar1;
  FUN_0049f930((int)piVar1);
  DAT_01eec99c = DAT_01eec99c + 1;
  local_4 = 0xffffffff;
  FUN_004b7a90((int *)&local_58);
  ExceptionList = local_c;
  return;
}


/* [VTable] CEGUI_UE3Texture virtual function #6
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

void __thiscall CEGUI_UE3Texture__vfunc_6(void *this,int param_1)

{
  short *psVar1;
  undefined **ppuVar2;
  wchar_t *pwVar3;
  int iVar4;
  undefined4 *puVar5;
  uint uVar6;
  int *piVar7;
  undefined4 uVar8;
  undefined4 uVar9;
  uint uVar10;
  uint uVar11;
  undefined4 uVar12;
  undefined4 uVar13;
  int iVar14;
  undefined **local_24;
  int local_20;
  int aiStack_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d4fd6;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_0041aab0(&local_24,L"SGW_UI.");
  local_4 = 0;
  if (*(uint *)(param_1 + 0x18) < 8) {
    psVar1 = (short *)(param_1 + 4);
  }
  else {
    psVar1 = *(short **)(param_1 + 4);
  }
  FUN_0041b4b0(&local_24,psVar1);
  ppuVar2 = local_24;
  if (local_20 == 0) {
    ppuVar2 = &PTR_017f8e10;
  }
  pwVar3 = wcsstr((wchar_t *)ppuVar2,L".material");
  if (pwVar3 == (wchar_t *)0x0) {
LAB_00a290b6:
    ppuVar2 = local_24;
    if (local_20 == 0) {
      ppuVar2 = &PTR_017f8e10;
    }
    if (DAT_01ee6588 == (undefined4 *)0x0) {
      DAT_01ee6588 = WxUnrealEdApp__unknown_008df850();
      WxUnrealEdApp__unknown_008dd840();
    }
    uVar6 = FUN_004a8e10(DAT_01ee6588,(undefined4 *)0x0,ppuVar2,(wchar_t *)0x0,0,(int *)0x0,1);
    *(uint *)((int)this + 8) = uVar6;
    if (uVar6 == 0) goto LAB_00a29107;
  }
  else {
    ppuVar2 = local_24;
    if (local_20 == 0) {
      ppuVar2 = &PTR_017f8e10;
    }
    iVar4 = (int)pwVar3 - (int)ppuVar2 >> 1;
    if (iVar4 < 0) goto LAB_00a290b6;
    puVar5 = FUN_00425e70(&local_24,aiStack_18,iVar4);
    local_4._0_1_ = 4;
    if (puVar5[1] == 0) {
      ppuVar2 = &PTR_017f8e10;
    }
    else {
      ppuVar2 = (undefined **)*puVar5;
    }
    if (DAT_01ee371c == (undefined4 *)0x0) {
      DAT_01ee371c = FUN_0073c750();
      FUN_0073b600();
    }
    uVar6 = FUN_004a8e10(DAT_01ee371c,(undefined4 *)0x0,ppuVar2,(wchar_t *)0x0,0,(int *)0x0,1);
    local_4 = (uint)local_4._1_3_ << 8;
    FUN_0041b420(aiStack_18);
    if (uVar6 == 0) goto LAB_00a29107;
    iVar14 = 0;
    uVar13 = 0;
    uVar12 = 0;
    uVar11 = 0;
    uVar10 = 0;
    uVar8 = 0;
    uVar9 = 0;
    iVar4 = *(int *)(uVar6 + 0x28);
    if (DAT_01ee371c == (undefined4 *)0x0) {
      DAT_01ee371c = FUN_0073c750();
      FUN_0073b600();
    }
    piVar7 = (int *)T_StaticClass_37(DAT_01ee371c,iVar4,uVar8,uVar9,uVar10,uVar11,uVar12,uVar13,
                                     iVar14);
    *(int **)((int)this + 0xc) = piVar7;
    if (piVar7 == (int *)0x0) goto LAB_00a29107;
    (**(code **)(*piVar7 + 0x154))();
    uVar6 = *(uint *)((int)this + 0xc);
  }
  FUN_0049f930(uVar6);
LAB_00a29107:
  local_4 = 0xffffffff;
  FUN_0041b420((int *)&local_24);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] CEGUI_UE3Texture virtual function #8
   VTable: vtable_CEGUI_UE3Texture at 0191ec34 */

undefined4 * __thiscall CEGUI_UE3Texture__vfunc_8(void *this,byte param_1)

{
  FUN_00a28cb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4
FD3DIncludeEnvironment__vfunc_0
          (int param_1,undefined4 param_2,LPCSTR param_3,undefined4 param_4,undefined4 *param_5,
          int *param_6)

{
  LPWSTR pWVar1;
  undefined **ppuVar2;
  int *piVar3;
  undefined4 *puVar4;
  int iVar5;
  char *_Dest;
  UINT *pUVar6;
  undefined **local_134;
  int local_130;
  undefined4 local_12c;
  undefined **local_128;
  int local_124;
  exception local_11c [12];
  undefined1 local_110 [4];
  undefined1 auStack_10c [128];
  undefined1 *puStack_8c;
  undefined1 *local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017143f3;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  pWVar1 = CEGUI__unknown_00423e50(local_110,param_3);
  local_4 = 0;
  FUN_0041aab0(&local_128,*(short **)(pWVar1 + 0x80));
  local_4 = CONCAT31(local_4._1_3_,2);
  if ((local_10 != local_110) && (local_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_10);
  }
  local_134 = (undefined **)0x0;
  local_130 = 0;
  local_12c = 0;
  local_4._0_1_ = 5;
  ppuVar2 = local_128;
  if (local_124 == 0) {
    ppuVar2 = &PTR_017f8e10;
  }
  FUN_0041aab0(local_11c,(short *)ppuVar2);
  local_4._0_1_ = 6;
  piVar3 = (int *)FConfigCacheIni__unknown_00ff6e10((void *)(param_1 + 4),(undefined4 *)local_11c);
  local_4._0_1_ = 5;
  FUN_0041b420((int *)local_11c);
  if (piVar3 == (int *)0x0) {
    puVar4 = FUN_0048ed10(&local_128,local_11c,1,2);
    local_4._0_1_ = 7;
    if (puVar4[1] == 0) {
      ppuVar2 = &PTR_017f8e10;
    }
    else {
      ppuVar2 = (undefined **)*puVar4;
    }
    iVar5 = _wcsicmp((wchar_t *)ppuVar2,L":\\");
    local_4._0_1_ = 5;
    FUN_0041b420((int *)local_11c);
    if (iVar5 == 0) {
      if (local_124 == 0) {
        local_128 = &PTR_017f8e10;
      }
      UEngine__unknown_00487680(&local_134,local_128,PTR_PTR_01dae7f8);
    }
    else {
      if (local_124 == 0) {
        local_128 = &PTR_017f8e10;
      }
      piVar3 = FUN_00541910((undefined4 *)local_11c,(short *)local_128);
      local_4._0_1_ = 8;
      FUN_0041a630(&local_134,piVar3);
      local_4._0_1_ = 5;
      FUN_0041b420((int *)local_11c);
    }
  }
  else {
    FUN_0041a630(&local_134,piVar3);
  }
  if (local_130 == 0) {
    iVar5 = 0;
  }
  else {
    iVar5 = local_130 + -1;
  }
  _Dest = (char *)scalable_malloc(iVar5 + 1);
  if (_Dest == (char *)0x0) {
    FUN_004099f0(local_11c);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_11c,(ThrowInfo *)&DAT_01d65cc8);
  }
  ppuVar2 = local_134;
  if (local_130 == 0) {
    ppuVar2 = &PTR_017f8e10;
  }
  pUVar6 = FUN_00423f40(local_110,(LPCWSTR)ppuVar2);
  local_4._0_1_ = 9;
  if (local_130 == 0) {
    iVar5 = 0;
  }
  else {
    iVar5 = local_130 + -1;
  }
  strncpy(_Dest,(char *)pUVar6[0x21],iVar5 + 1);
  local_4 = CONCAT31(local_4._1_3_,5);
  if ((puStack_8c != auStack_10c) && (puStack_8c != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(puStack_8c);
  }
  *param_5 = _Dest;
  if (local_130 == 0) {
    iVar5 = 0;
  }
  else {
    iVar5 = local_130 + -1;
  }
  *param_6 = iVar5;
  local_4 = CONCAT31(local_4._1_3_,2);
  FUN_0041b420((int *)&local_134);
  local_4 = 0xffffffff;
  FUN_0041b420((int *)&local_128);
  ExceptionList = pvStack_c;
  return 0;
}


undefined4 * __thiscall CEGUI_WindowRendererFactory__vfunc_0(void *this,byte param_1)

{
  FUN_01164920(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


void __thiscall CEGUI_RichTextEditboxFactory__vfunc_0(void *this,void *param_1)

{
  void *this_00;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0175e9fb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x278,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_01169180(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_RichTextItemEntryFactory__vfunc_0(void *this,void *param_1)

{
  void *this_00;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0175e9fb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x268,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_01166da0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


undefined4 * __thiscall CEGUI_RichTextEditboxRendererWRFactory__vfunc_0(void *this,byte param_1)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0175ea88;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_01164920(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar1);
  }
  ExceptionList = local_c;
  return this;
}


undefined4 * __thiscall CEGUI_ItemEntry__vfunc_0(void *this,byte param_1)

{
  FUN_01165290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_RichTextEventArgs__vfunc_0(void *this,byte param_1)

{
  FUN_011659a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_RichTextItemEntry__vfunc_0(void *this,byte param_1)

{
  FUN_01166ee0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_RichTextEditboxProperties_CaratIndex__vfunc_0(void *this,byte param_1)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0175ef88;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_01167ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar1);
  }
  ExceptionList = local_c;
  return this;
}


undefined4 * __thiscall CEGUI_RichTextEditbox__vfunc_0(void *this,byte param_1)

{
  FUN_01169300(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_RichTextItemEntryRenderer__vfunc_0(void *this,byte param_1)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0175f250;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_011fbc40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar1);
  }
  ExceptionList = local_c;
  return this;
}


undefined4 * __thiscall CEGUI_RichTextEditboxRenderer__vfunc_0(void *this,byte param_1)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0175f2a8;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_011fbc40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar1);
  }
  ExceptionList = local_c;
  return this;
}


undefined4 * __thiscall CEGUI_PropertyReceiver__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::PropertyReceiver::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_PropertySet__vfunc_0(void *this,byte param_1)

{
  FUN_011a0740(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Font__vfunc_0(void *this,byte param_1)

{
  FUN_011a0a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Window__vfunc_0(void *this,byte param_1)

{
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Font_xmlHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Font_xmlHandler::vftable;
  FUN_01209e60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_GUILayout_xmlHandler__vfunc_0(void *this,byte param_1)

{
  FUN_011ac570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Config_xmlHandler__vfunc_0(void *this,byte param_1)

{
  FUN_011af1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_System__vfunc_0(void *this,byte param_1)

{
  FUN_011b1cd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Imageset_xmlHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Imageset_xmlHandler::vftable;
  FUN_01209e60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Scheme_xmlHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Scheme_xmlHandler::vftable;
  FUN_01209e60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FalagardComponentBase__vfunc_0(void *this,byte param_1)

{
  FUN_01217ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ImageryComponent__vfunc_0(void *this,byte param_1)

{
  FUN_011b8840(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_TextComponent__vfunc_0(void *this,byte param_1)

{
  FUN_011b89f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_PropertyDefinition__vfunc_0(void *this,byte param_1)

{
  FUN_011b8fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_PropertyLinkDefinition__vfunc_0(void *this,byte param_1)

{
  FUN_011b9100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Renderer__vfunc_0(void *this,byte param_1)

{
  FUN_011bf2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_DefaultResourceProvider__vfunc_0(void *this,byte param_1)

{
  FUN_011bf4c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_UE3Renderer virtual function #23
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __fastcall CEGUI_UE3Renderer__vfunc_23(int param_1)

{
  undefined4 *puVar1;
  char *local_1c;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_1c = (char *)scalable_malloc(0x2c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (local_1c == (char *)0x0) {
    local_1c = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&local_1c);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  puVar1 = FUN_011bf440((undefined4 *)local_1c);
  *(undefined4 **)(param_1 + 0x18) = puVar1;
  ExceptionList = local_c;
  return;
}


undefined4 * __thiscall CEGUI_DragContainer__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::DragContainer::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::DragContainer::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CEGUI_EventArgs (slot 0) */

undefined4 * __thiscall CEGUI_ActivationEventArgs__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::EventArgs::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_MouseCursor__vfunc_0(void *this,byte param_1)

{
  CEGUI_MouseCursor_17(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_GlobalEventSet__vfunc_0(void *this,byte param_1)

{
  CEGUI_GlobalEventSet_2(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_UE3Renderer virtual function #4
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c
   [String discovery] References: "EventSet::subscribeScriptedEvent" */

undefined4 __thiscall
CEGUI_UE3Renderer__vfunc_4(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  uint uVar1;
  int iVar2;
  int *piVar3;
  undefined1 local_a0 [4];
  undefined2 local_9c;
  undefined4 local_8c;
  undefined4 local_88;
  undefined1 local_84 [4];
  undefined2 local_80;
  undefined4 local_70;
  undefined4 local_6c;
  undefined1 local_68 [76];
  void *pvStack_1c;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01765fa6;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffff54;
  ExceptionList = &local_c;
  iVar2 = FUN_011aeb20();
  piVar3 = (int *)FUN_011add40(iVar2);
  if (piVar3 == (int *)0x0) {
    local_88 = 7;
    local_8c = 0;
    local_9c = 0;
    FUN_00438520(local_a0,L"..\\..\\..\\src\\CEGUIEventSet.cpp",0x1e);
    local_4 = 0;
    local_6c = 7;
    local_70 = 0;
    local_80 = 0;
    FUN_00438520(local_84,L"[EventSet::subscribeScriptedEvent] No scripting module is available",
                 0x43);
    local_4 = CONCAT31(local_4._1_3_,1);
    CEGUI_InvalidRequestException(local_68,local_84,local_a0,0x7e);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_68,(ThrowInfo *)&DAT_01c724e4);
  }
  (**(code **)(*piVar3 + 0x20))(param_1,this,param_2,param_3,uVar1);
  ExceptionList = pvStack_1c;
  return param_1;
}


undefined4 * __thiscall CEGUI_EventSet__vfunc_0(void *this,byte param_1)

{
  CEGUI__unknown_011c2b80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CEGUI_UE3Renderer virtual function #2
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

int * __thiscall CEGUI_UE3Renderer__vfunc_2(void *this,int *param_1,void *param_2)

{
  void *this_00;
  int *piVar1;
  char *pcVar2;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_01766071;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  pcVar2 = &stack0x0000000c;
  local_4 = 1;
  piVar1 = param_1;
  this_00 = (void *)FUN_011c2de0(this,param_2,'\x01');
  FUN_0120e930(this_00,piVar1,pcVar2);
  local_4 = local_4 & 0xffffff00;
  _00___FRawStaticIndexBuffer__vfunc_0();
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] CEGUI_UE3Renderer virtual function #1
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

int * __thiscall
CEGUI_UE3Renderer__vfunc_1(void *this,int *param_1,void *param_2,undefined **param_3)

{
  void *this_00;
  int *piVar1;
  char *pcVar2;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_017660b1;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  pcVar2 = &stack0x00000010;
  local_4 = 1;
  piVar1 = param_1;
  this_00 = (void *)FUN_011c2de0(this,param_2,'\x01');
  FUN_0120e6e0(this_00,piVar1,param_3,pcVar2);
  local_4 = local_4 & 0xffffff00;
  _00___FRawStaticIndexBuffer__vfunc_0();
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] CEGUI_UE3Renderer virtual function #5
   VTable: vtable_CEGUI_UE3Renderer at 018fa34c */

void __thiscall CEGUI_UE3Renderer__vfunc_5(void *this,void *param_1,int param_2,undefined4 param_3)

{
  int *piVar1;
  
  if (*(char *)((int)this + 0x11) == '\0') {
    piVar1 = (int *)FUN_011c2160();
    (**(code **)(*piVar1 + 0x14))(param_1,param_2,param_3);
    FUN_011c2ea0(this,param_1,param_2);
  }
  return;
}


undefined4 * __thiscall CEGUI_Tree__vfunc_0(void *this,byte param_1)

{
  FUN_011c4a10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Tooltip__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Tooltip::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Tooltip::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_TabControl__vfunc_0(void *this,byte param_1)

{
  FUN_011c9f30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Spinner__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Spinner::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Spinner::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Slider__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Slider::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Slider::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ScrolledContainer__vfunc_0(void *this,byte param_1)

{
  FUN_011cd9d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Scrollbar__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Scrollbar::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Scrollbar::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_EditboxWindowRenderer__vfunc_0(void *this,byte param_1)

{
  FUN_011fbc40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ScrollablePane__vfunc_0(void *this,byte param_1)

{
  FUN_011cf2c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ProgressBar__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ProgressBar::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ProgressBar::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_MultiLineEditbox__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::MultiLineEditbox::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::MultiLineEditbox::vftable;
  if (*(int *)((int)this + 0x260) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(int *)((int)this + 0x260));
  }
  *(undefined4 *)((int)this + 0x260) = 0;
  *(undefined4 *)((int)this + 0x264) = 0;
  *(undefined4 *)((int)this + 0x268) = 0;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_CheckboxProperties_Selected__vfunc_0(void *this,byte param_1)

{
  FUN_01167ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_MultiColumnList__vfunc_0(void *this,byte param_1)

{
  FUN_011dc660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ListHeaderSegment__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ListHeaderSegment::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ListHeaderSegment::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ListHeader__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ListHeader::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ListHeader::vftable;
  if (*(int *)((int)this + 0x240) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(int *)((int)this + 0x240));
  }
  *(undefined4 *)((int)this + 0x240) = 0;
  *(undefined4 *)((int)this + 0x244) = 0;
  *(undefined4 *)((int)this + 0x248) = 0;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Listbox__vfunc_0(void *this,byte param_1)

{
  FUN_011e2390(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ItemListbox__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ItemListbox::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ItemListbox::vftable;
  FUN_011e5470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ScrolledItemListBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ScrolledItemListBase::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ScrolledItemListBase::vftable;
  FUN_011e6bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CEGUI_SlotFunctorBase (slot 0) */

undefined4 * __thiscall CEGUI_CEGUI_VCombobox___MemberFunctionSlot__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::SlotFunctorBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_MenuBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::MenuBase::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::MenuBase::vftable;
  FUN_011e6bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ItemListBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ItemListBase::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ItemListBase::vftable;
  if (*(int *)((int)this + 0x240) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(int *)((int)this + 0x240));
  }
  *(undefined4 *)((int)this + 0x240) = 0;
  *(undefined4 *)((int)this + 0x244) = 0;
  *(undefined4 *)((int)this + 0x248) = 0;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_MenuItem__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ItemEntry::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ItemEntry::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FrameWindow__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::FrameWindow::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::FrameWindow::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Editbox__vfunc_0(void *this,byte param_1)

{
  FUN_011ec070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Combobox__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Combobox::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Combobox::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_TabButton__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::TabButton::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::TabButton::vftable;
  FUN_01222950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_RadioButton__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::RadioButton::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::RadioButton::vftable;
  FUN_01222950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Thumb__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Thumb::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Thumb::vftable;
  FUN_011f1950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_PushButton__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::PushButton::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::PushButton::vftable;
  FUN_01222950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Checkbox__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Checkbox::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Checkbox::vftable;
  FUN_01222950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ListboxItem__vfunc_0(void *this,byte param_1)

{
  FUN_011f2140(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ListboxTextItem__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ListboxTextItem::vftable;
  FUN_011f2140(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ClippedContainer__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ClippedContainer::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ClippedContainer::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Titlebar__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Titlebar::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Titlebar::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_PopupMenu__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::PopupMenu::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::PopupMenu::vftable;
  FUN_011e6260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ExternalWindow__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ExternalWindow::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ExternalWindow::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Logger__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Logger::vftable;
  if (DAT_01f04314 == 0) {
    _wassert(L"ms_Singleton",
             L"c:\\build\\qa\\sgw\\working\\development\\external\\cegui\\include\\CEGUISingleton.h"
             ,0x4d);
  }
  DAT_01f04314 = 0;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ComboDropList__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ComboDropList::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ComboDropList::vftable;
  FUN_011e2390(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Menubar__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::Menubar::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::Menubar::vftable;
  FUN_011e6260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_GroupBox__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::GroupBox::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::GroupBox::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_AbsoluteDim__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::BaseDim::vftable;
  if (*(undefined4 **)((int)this + 8) != (undefined4 *)0x0) {
    (**(code **)**(undefined4 **)((int)this + 8))(1);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ImageDim__vfunc_0(void *this,byte param_1)

{
  FUN_012044b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_WidgetDim__vfunc_0(void *this,byte param_1)

{
  FUN_01204690(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FontDim__vfunc_0(void *this,byte param_1)

{
  FUN_01204890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_XMLSerializer__vfunc_0(void *this,byte param_1)

{
  FUN_01206ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


void __fastcall CEGUI_XMLAttributes__vfunc_0(undefined4 *param_1)

{
  undefined4 *this;
  undefined4 local_8 [2];
  
  this = param_1 + 1;
  *param_1 = CEGUI::XMLAttributes::vftable;
  FUN_0043a800(this,local_8,this,*(int **)param_1[2],this,(int *)param_1[2]);
                    /* WARNING: Subroutine does not return */
  scalable_free(param_1[2]);
}


undefined4 * __thiscall CEGUI_XMLHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::XMLHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_PixmapFont__vfunc_0(void *this,byte param_1)

{
  FUN_0120a240(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FreeTypeFont__vfunc_0(void *this,byte param_1)

{
  FUN_0120b790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_XMLParser__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::XMLParser::vftable;
  if (7 < *(uint *)((int)this + 0x1c)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 *)((int)this + 8));
  }
  *(undefined4 *)((int)this + 0x1c) = 7;
  *(undefined4 *)((int)this + 0x18) = 0;
  *(undefined2 *)((int)this + 8) = 0;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


void __thiscall CEGUI_GUISheetFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x23c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_01211fb0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_DragContainerFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x27c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011c10f0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ScrolledContainerFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x25c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011cdac0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ClippedContainerFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x250,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f3590(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_CheckboxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x284,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f1ba0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ComboDropListFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x260,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f4ea0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ComboboxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x240,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011ee2b0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_EditboxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x278,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011ed930(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_FrameWindowFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x274,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011e9c20(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ItemEntryFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x244,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011e8bc0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ListHeaderFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x260,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011df230(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ListHeaderSegmentFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x26c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011de000(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ListboxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(600,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011e1f80(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_MenuItemFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x250,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011e7e80(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_MenubarFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x270,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f52c0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_MultiColumnListFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x26c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011dbb00(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_MultiLineEditboxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x278,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011d26b0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_PopupMenuFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x284,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f3dd0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ProgressBarFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x244,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011d1280(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_PushButtonFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x280,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f1920(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_RadioButtonFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x288,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f05b0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ScrollablePaneFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x270,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011cff60(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ScrollbarFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x250,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011cde30(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_SliderFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x248,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011cbe70(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_SpinnerFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x250,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011ca430(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_TabButtonFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x288,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f0280(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_TabControlFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x274,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011ca0d0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ThumbFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x2a0,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f11d0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_TitlebarFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x25c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f3a20(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_TooltipFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x254,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011c6920(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ItemListboxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x274,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011e4640(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_GroupBoxFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x23c,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f56d0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_TreeFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x278,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  Window__unknown_011c45b0(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


void __thiscall CEGUI_ExternalWindowFactory__vfunc_0(void *this,char *param_1)

{
  void *this_00;
  undefined **local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0176b1eb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this_00 = (void *)scalable_malloc(0x240,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this_00 == (void *)0x0) {
    param_1 = "scalable_malloc failed";
    std::exception::exception((exception *)local_18,&param_1);
    local_18[0] = std::bad_alloc::vftable;
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_011f4b90(this_00,(void *)((int)this + 4),param_1);
  ExceptionList = local_c;
  return;
}


undefined4 * __thiscall CEGUI_GUISheet__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::GUISheet::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::GUISheet::vftable;
  FUN_011aaef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_DefaultLogger__vfunc_0(void *this,byte param_1)

{
  CEGUI_Logger_7(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FactoryModule__vfunc_0(void *this,byte param_1)

{
  int iVar1;
  
  iVar1 = *(int *)((int)this + 0xc);
  *(undefined ***)this = CEGUI::FactoryModule::vftable;
  if (iVar1 != 0) {
    FUN_0120ed60(iVar1);
                    /* WARNING: Subroutine does not return */
    scalable_free(iVar1);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_Falagard_xmlHandler__vfunc_0(void *this,byte param_1)

{
  FUN_0121e5d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_ButtonBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::ButtonBase::vftable;
  *(undefined ***)((int)this + 0x10) = CEGUI::ButtonBase::vftable;
  FUN_011c05c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FalagardButtonWRFactory__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::WindowRendererFactory::vftable;
  if (7 < *(uint *)((int)this + 0x1c)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 *)((int)this + 8));
  }
  *(undefined4 *)((int)this + 0x1c) = 7;
  *(undefined4 *)((int)this + 0x18) = 0;
  *(undefined2 *)((int)this + 8) = 0;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FalagardTree__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::FalagardTree::vftable;
  FUN_011fbc40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FalagardListHeader__vfunc_0(void *this,byte param_1)

{
  FUN_01228900(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_FalagardStaticText__vfunc_0(void *this,byte param_1)

{
  FUN_0122a790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_TinyXMLDocument__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUITinyXML::TiXmlDocument::vftable;
  if (*(undefined4 **)((int)this + 0x34) != &DAT_01f11c48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 **)((int)this + 0x34));
  }
  FUN_015569e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUI_TinyXMLParser__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUI::TinyXMLParser::vftable;
  FUN_0120eca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUITinyXML::TiXmlBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlAttribute__vfunc_0(void *this,byte param_1)

{
  if (*(undefined4 **)((int)this + 0x18) != &DAT_01f11c48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 **)((int)this + 0x18));
  }
  if (*(undefined4 **)((int)this + 0x14) != &DAT_01f11c48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 **)((int)this + 0x14));
  }
  *(undefined ***)this = CEGUITinyXML::TiXmlBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlComment__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUITinyXML::TiXmlComment::vftable;
  FUN_015569e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlText__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUITinyXML::TiXmlText::vftable;
  FUN_015569e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlDeclaration__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUITinyXML::TiXmlDeclaration::vftable;
  if (*(undefined4 **)((int)this + 0x34) != &DAT_01f11c48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 **)((int)this + 0x34));
  }
  if (*(undefined4 **)((int)this + 0x30) != &DAT_01f11c48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 **)((int)this + 0x30));
  }
  if (*(undefined4 **)((int)this + 0x2c) != &DAT_01f11c48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 **)((int)this + 0x2c));
  }
  FUN_015569e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlUnknown__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CEGUITinyXML::TiXmlUnknown::vftable;
  FUN_015569e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlNode__vfunc_0(void *this,byte param_1)

{
  FUN_015569e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CEGUITinyXML_TiXmlElement__vfunc_0(void *this,byte param_1)

{
  FUN_015577f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* === Standalone functions (14) === */

/* [VTable] CEGUI_ScriptException virtual function #1
   VTable: vtable_CEGUI_ScriptException at 018f9fd8
   Also in vtable: CEGUI_InvalidRequestException (slot 1)
   Also in vtable: CEGUI_FileIOException (slot 1)
   Also in vtable: CEGUI_AlreadyExistsException (slot 1)
   Also in vtable: CEGUI_GenericException (slot 1)
   Also in vtable: CEGUI_UnknownObjectException (slot 1) */

void __thiscall CEGUI_ScriptException__vfunc_1(void *this)

{
  int *piVar1;
  char unaff_BL;
  int unaff_EBP;
  int *unaff_ESI;
  int *piVar2;
  int *unaff_EDI;
  void *in_stack_00000010;
  int iStack00000014;
  undefined4 uStack00000064;
  
  if (unaff_EDI != *(int **)(*(int *)((int)this + 4) + 4)) {
    do {
      piVar2 = unaff_ESI;
      if ((char)unaff_EDI[0x1c] != unaff_BL) break;
      piVar1 = (int *)*piVar2;
      if (unaff_EDI == piVar1) {
        piVar1 = (int *)piVar2[2];
        if ((char)piVar1[0x1c] == '\0') {
          *(char *)(piVar1 + 0x1c) = unaff_BL;
          *(undefined1 *)(piVar2 + 0x1c) = 0;
          HInterpTrackKeyHandleProxy__unknown_011b2380(this,(int)piVar2);
          piVar1 = (int *)piVar2[2];
          this = in_stack_00000010;
        }
        if (*(char *)((int)piVar1 + 0x71) == '\0') {
          if ((*(char *)(*piVar1 + 0x70) != unaff_BL) || (*(char *)(piVar1[2] + 0x70) != unaff_BL))
          {
            if (*(char *)(piVar1[2] + 0x70) == unaff_BL) {
              *(char *)(*piVar1 + 0x70) = unaff_BL;
              *(undefined1 *)(piVar1 + 0x1c) = 0;
              HInterpTrackKeyHandleProxy__unknown_00423cc0(this,piVar1);
              piVar1 = (int *)piVar2[2];
              this = in_stack_00000010;
            }
            *(char *)(piVar1 + 0x1c) = (char)piVar2[0x1c];
            *(char *)(piVar2 + 0x1c) = unaff_BL;
            *(char *)(piVar1[2] + 0x70) = unaff_BL;
            HInterpTrackKeyHandleProxy__unknown_011b2380(this,(int)piVar2);
            break;
          }
LAB_004500f4:
          *(undefined1 *)(piVar1 + 0x1c) = 0;
        }
      }
      else {
        if ((char)piVar1[0x1c] == '\0') {
          *(char *)(piVar1 + 0x1c) = unaff_BL;
          *(undefined1 *)(piVar2 + 0x1c) = 0;
          HInterpTrackKeyHandleProxy__unknown_00423cc0(this,piVar2);
          piVar1 = (int *)*piVar2;
          this = in_stack_00000010;
        }
        if (*(char *)((int)piVar1 + 0x71) == '\0') {
          if ((*(char *)(piVar1[2] + 0x70) == unaff_BL) && (*(char *)(*piVar1 + 0x70) == unaff_BL))
          goto LAB_004500f4;
          if (*(char *)(*piVar1 + 0x70) == unaff_BL) {
            *(char *)(piVar1[2] + 0x70) = unaff_BL;
            *(undefined1 *)(piVar1 + 0x1c) = 0;
            HInterpTrackKeyHandleProxy__unknown_011b2380(this,(int)piVar1);
            piVar1 = (int *)*piVar2;
            this = in_stack_00000010;
          }
          *(char *)(piVar1 + 0x1c) = (char)piVar2[0x1c];
          *(char *)(piVar2 + 0x1c) = unaff_BL;
          *(char *)(*piVar1 + 0x70) = unaff_BL;
          HInterpTrackKeyHandleProxy__unknown_00423cc0(this,piVar2);
          break;
        }
      }
      unaff_ESI = (int *)piVar2[1];
      unaff_EDI = piVar2;
    } while (piVar2 != *(int **)(*(int *)((int)this + 4) + 4));
  }
  *(char *)(unaff_EDI + 0x1c) = unaff_BL;
  iStack00000014 = unaff_EBP + 0xc;
  uStack00000064 = 0xffffffff;
  FUN_0044f3e0(unaff_EBP + 0x10);
                    /* WARNING: Subroutine does not return */
  scalable_free();
}


/* [VTable] CEGUI_ScriptException virtual function #2
   VTable: vtable_CEGUI_ScriptException at 018f9fd8
   Also in vtable: CEGUI_InvalidRequestException (slot 2)
   Also in vtable: CEGUI_FileIOException (slot 2)
   Also in vtable: CEGUI_AlreadyExistsException (slot 2)
   Also in vtable: CEGUI_GenericException (slot 2)
   Also in vtable: CEGUI_UnknownObjectException (slot 2) */

void __fastcall CEGUI_ScriptException__vfunc_2(undefined4 param_1,int param_2)

{
  int iVar1;
  int *piVar2;
  undefined **ppuVar3;
  void *pvVar4;
  int *unaff_EBX;
  void *unaff_EBP;
  wchar_t *pwVar5;
  void *unaff_EDI;
  int in_stack_0000001c;
  undefined4 in_stack_00000020;
  undefined4 in_stack_00000024;
  undefined4 in_stack_00000028;
  undefined4 in_stack_0000002c;
  undefined4 in_stack_00000030;
  undefined4 in_stack_00000034;
  void *in_stack_00000a5c;
  int *piVar6;
  
  if (*(int **)(param_2 + 0x2cc) != unaff_EBX) {
    iVar1 = UGameEngine__unknown_00561a40(unaff_EBP,L"Listen");
    pwVar5 = L"game-ini:Engine.GameInfo.DefaultGame";
    if (iVar1 == 0) goto LAB_00550069;
  }
  pwVar5 = L"game-ini:Engine.GameInfo.DefaultServerGame";
LAB_00550069:
  if (DAT_01ee39d4 == unaff_EBX) {
    DAT_01ee39d4 = FUN_00770190();
    FUN_00776d30();
  }
  piVar2 = FUN_004ac440(DAT_01ee39d4,unaff_EBX,(undefined **)pwVar5,(wchar_t *)unaff_EBX,
                        (uint)unaff_EBX,unaff_EBX);
  if (piVar2 == unaff_EBX) {
    piVar2 = DAT_01ee39d4;
    if (DAT_01ee39d4 == unaff_EBX) {
      DAT_01ee39d4 = FUN_00770190();
      FUN_00776d30();
      piVar2 = DAT_01ee39d4;
    }
  }
  else {
    FUN_0041aab0(&stack0x00000038,(short *)&stack0x0000025c);
    if (*(int **)((int)unaff_EBP + 0x20) == unaff_EBX) {
      ppuVar3 = &PTR_017f8e10;
    }
    else {
      ppuVar3 = *(undefined ***)((int)unaff_EBP + 0x1c);
    }
    FUN_0041aab0(&stack0x00000044,(short *)ppuVar3);
    pvVar4 = FUN_004b2d80(piVar2,(int)unaff_EBX);
    piVar2 = (int *)&stack0x00000038;
    piVar6 = (int *)&stack0x00000044;
    pvVar4 = (void *)FUN_00554b00((int)pvVar4);
    piVar2 = (int *)FUN_00555c10(pvVar4,piVar6,piVar2);
    FUN_0041b420((int *)&stack0x00000044);
    FUN_0041b420((int *)&stack0x00000038);
  }
  in_stack_00000020 = 0;
  in_stack_00000024 = 0;
  in_stack_00000028 = 0;
  piVar2 = FUN_00876970(unaff_EDI,piVar2,(int)unaff_EBX,unaff_EBX,(undefined8 *)&stack0x00000020,
                        (undefined8 *)&stack0x0000002c,unaff_EBX,(int)unaff_EBX,(int)unaff_EBX,
                        unaff_EBX,unaff_EBX,(int)unaff_EBX);
  *(int **)(in_stack_0000001c + 0x35c) = piVar2;
  if (piVar2 == unaff_EBX) {
    FUN_00486000("Info->Game!=NULL",".\\Src\\UnWorld.cpp",0x892);
  }
  FUN_0041b420((int *)&stack0x00000050);
  ExceptionList = in_stack_00000a5c;
  return;
}


/* Also in vtable: CEGUI_ScriptException (slot 0)
   Also in vtable: CEGUI_InvalidRequestException (slot 0)
   Also in vtable: CEGUI_FileIOException (slot 0)
   Also in vtable: CEGUI_AlreadyExistsException (slot 0)
   Also in vtable: CEGUI_GenericException (slot 0)
   Also in vtable: CEGUI_UnknownObjectException (slot 0) */

undefined4 * __thiscall CEGUI_AlreadyExistsException__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016c5698;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_011c2480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::InvalidRequestException""
   String at: 018fa9d0 */

undefined4 * __thiscall
CEGUI_InvalidRequestException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  uint uVar1;
  wchar_t local_30 [2];
  void *local_2c;
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016c5ec0;
  pvStack_c = ExceptionList;
  local_10 = 7;
  local_30[0] = L'\0';
  local_30[1] = L'\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  local_2c = this;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,local_30);
  uVar1 = std::char_traits<wchar_t>::length(L"CEGUI::InvalidRequestException");
  FUN_00438520(auStack_28,L"CEGUI::InvalidRequestException",uVar1);
  uStack_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,auStack_28,param_2,param_3);
  uStack_4 = CONCAT31(uStack_4._1_3_,2);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 7;
  param_3 = 0;
  local_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,(wchar_t *)&param_3);
  *(undefined ***)this = CEGUI::InvalidRequestException::vftable;
  ExceptionList = pvStack_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::FileIOException""
   String at: 018faa18 */

undefined4 * __thiscall
CEGUI_FileIOException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  uint uVar1;
  wchar_t local_30 [2];
  void *local_2c;
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016c5ec0;
  pvStack_c = ExceptionList;
  local_10 = 7;
  local_30[0] = L'\0';
  local_30[1] = L'\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  local_2c = this;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,local_30);
  uVar1 = std::char_traits<wchar_t>::length(L"CEGUI::FileIOException");
  FUN_00438520(auStack_28,L"CEGUI::FileIOException",uVar1);
  uStack_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,auStack_28,param_2,param_3);
  uStack_4 = CONCAT31(uStack_4._1_3_,2);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 7;
  param_3 = 0;
  local_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,(wchar_t *)&param_3);
  *(undefined ***)this = CEGUI::FileIOException::vftable;
  ExceptionList = pvStack_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::AlreadyExistsException""
   String at: 018faa50 */

undefined4 * __thiscall
CEGUI_AlreadyExistsException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  uint uVar1;
  wchar_t local_30 [2];
  void *local_2c;
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016c5ec0;
  pvStack_c = ExceptionList;
  local_10 = 7;
  local_30[0] = L'\0';
  local_30[1] = L'\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  local_2c = this;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,local_30);
  uVar1 = std::char_traits<wchar_t>::length(L"CEGUI::AlreadyExistsException");
  FUN_00438520(auStack_28,L"CEGUI::AlreadyExistsException",uVar1);
  uStack_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,auStack_28,param_2,param_3);
  uStack_4 = CONCAT31(uStack_4._1_3_,2);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 7;
  param_3 = 0;
  local_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,(wchar_t *)&param_3);
  *(undefined ***)this = CEGUI::AlreadyExistsException::vftable;
  ExceptionList = pvStack_c;
  return this;
}


/* [VTable] CEGUI_Texture virtual function #2
   VTable: vtable_CEGUI_Texture at 0191ec0c
   Also in vtable: CEGUI_UE3Texture (slot 2) */

float10 __fastcall CEGUI_Texture__vfunc_2(int *param_1)

{
  ushort uVar1;
  
  uVar1 = (**(code **)(*param_1 + 4))(param_1);
  return (float10)1 / (float10)uVar1;
}


/* [VTable] CEGUI_Texture virtual function #5
   VTable: vtable_CEGUI_Texture at 0191ec0c
   Also in vtable: CEGUI_UE3Texture (slot 5) */

float10 __fastcall CEGUI_Texture__vfunc_5(int *param_1)

{
  ushort uVar1;
  
  uVar1 = (**(code **)(*param_1 + 0x10))(param_1);
  return (float10)1 / (float10)uVar1;
}


/* [String discovery] Debug string: "u"CEGUI::GenericException""
   String at: 0191ed14 */

undefined4 * __thiscall
CEGUI_GenericException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  uint uVar1;
  wchar_t local_30 [2];
  void *local_2c;
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016c5ec0;
  pvStack_c = ExceptionList;
  local_10 = 7;
  local_30[0] = L'\0';
  local_30[1] = L'\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  local_2c = this;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,local_30);
  uVar1 = std::char_traits<wchar_t>::length(L"CEGUI::GenericException");
  FUN_00438520(auStack_28,L"CEGUI::GenericException",uVar1);
  uStack_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,auStack_28,param_2,param_3);
  uStack_4 = CONCAT31(uStack_4._1_3_,2);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 7;
  param_3 = 0;
  local_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_24,(wchar_t *)&param_3);
  *(undefined ***)this = CEGUI::GenericException::vftable;
  ExceptionList = pvStack_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::UnknownObjectException""
   String at: 0195fc10 */

undefined4 * __thiscall
CEGUI_UnknownObjectException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  undefined1 local_28 [4];
  uint local_24;
  undefined4 local_14;
  uint local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016c5ec0;
  local_c = ExceptionList;
  local_10 = 7;
  local_14 = 0;
  local_24 = local_24 & 0xffff0000;
  ExceptionList = &local_c;
  FUN_00438520(local_28,L"CEGUI::UnknownObjectException",0x1d);
  local_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,local_28,param_2,param_3);
  local_4 = CONCAT31(local_4._1_3_,2);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24);
  }
  *(undefined ***)this = CEGUI::UnknownObjectException::vftable;
  ExceptionList = local_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::NullObjectException""
   String at: 01aab4f0 */

undefined4 * __thiscall
CEGUI_NullObjectException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  uint uVar1;
  undefined1 local_28 [4];
  uint local_24;
  undefined4 local_14;
  uint local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017634e0;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffd0;
  ExceptionList = &local_c;
  local_10 = 7;
  local_14 = 0;
  local_24 = local_24 & 0xffff0000;
  FUN_00438520(local_28,L"CEGUI::NullObjectException",0x1a);
  local_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,local_28,param_2,param_3);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24,uVar1);
  }
  *(undefined ***)this = CEGUI::NullObjectException::vftable;
  ExceptionList = local_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::System::default__auto_tooltip__""
   String at: 01aad240 */

void __thiscall CEGUI_System_default_auto_tooltip(void *this,void *param_1)

{
  uint *puVar1;
  undefined4 uVar2;
  uint uStack_c4;
  undefined1 local_b4 [4];
  uint local_b0;
  undefined4 local_a0;
  uint local_9c;
  undefined1 local_98 [4];
  uint local_94;
  undefined4 local_84;
  uint local_80;
  undefined1 *local_7c;
  void *local_78;
  undefined1 *puStack_74;
  undefined4 local_70;
  void *local_6c;
  void *local_68;
  uint local_8;
  
  local_70 = 0xffffffff;
  puStack_74 = &LAB_01764480;
  local_78 = ExceptionList;
  uStack_c4 = DAT_01e7f9a0 ^ (uint)&local_6c;
  local_7c = (undefined1 *)&uStack_c4;
  ExceptionList = &local_78;
  local_6c = param_1;
  puVar1 = &uStack_c4;
  local_68 = this;
  local_8 = uStack_c4;
  if ((*(int *)((int)this + 0xac) != 0) &&
     (puVar1 = &uStack_c4, *(char *)((int)this + 0xb0) != '\0')) {
    puVar1 = &uStack_c4;
    if (DAT_01f010c0 == (void *)0x0) {
      _wassert(L"ms_Singleton",
               L"c:\\build\\qa\\sgw\\working\\development\\external\\cegui\\include\\CEGUISingleton.h"
               ,0x4f);
      puVar1 = (uint *)local_7c;
    }
    local_7c = (undefined1 *)puVar1;
    FUN_011ace70(DAT_01f010c0,*(int *)((int)this + 0xac));
    puVar1 = (uint *)local_7c;
  }
  local_7c = (undefined1 *)puVar1;
  if (*(int *)((int)param_1 + 0x14) == 0) {
    *(undefined4 *)((int)this + 0xac) = 0;
    *(undefined1 *)((int)this + 0xb0) = 0;
  }
  else {
    local_70 = 0;
    local_9c = 7;
    local_a0 = 0;
    local_b0 = local_b0 & 0xffff0000;
    FUN_00438520(local_b4,(wchar_t *)&PTR_017f8e10,0);
    local_70._0_1_ = 1;
    local_80 = 7;
    local_84 = 0;
    local_94 = local_94 & 0xffff0000;
    FUN_00438520(local_98,L"CEGUI::System::default__auto_tooltip__",0x26);
    local_70 = CONCAT31(local_70._1_3_,2);
    if (DAT_01f010c0 == (void *)0x0) {
      _wassert(L"ms_Singleton",
               L"c:\\build\\qa\\sgw\\working\\development\\external\\cegui\\include\\CEGUISingleton.h"
               ,0x4f);
    }
    uVar2 = WindowManager_createWindow(DAT_01f010c0,local_6c,(int)local_98,local_b4);
    *(undefined4 *)((int)this + 0xac) = uVar2;
    if (7 < local_80) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_94);
    }
    local_80 = 7;
    local_84 = 0;
    local_94 = local_94 & 0xffff0000;
    if (7 < local_9c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_b0);
    }
    *(undefined1 *)((int)this + 0xb0) = 1;
    *(undefined1 *)(*(int *)((int)this + 0xac) + 0x178) = 0;
  }
  ExceptionList = local_78;
  __security_check_cookie(local_8 ^ (uint)&local_6c);
  return;
}


/* [String discovery] Debug string: "u"CEGUI::RendererException""
   String at: 01ac1c08 */

undefined4 * __thiscall
CEGUI_RendererException(void *this,void *param_1,void *param_2,undefined4 param_3)

{
  uint uVar1;
  undefined1 local_28 [4];
  uint local_24;
  undefined4 local_14;
  uint local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017634e0;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffd0;
  ExceptionList = &local_c;
  local_10 = 7;
  local_14 = 0;
  local_24 = local_24 & 0xffff0000;
  FUN_00438520(local_28,L"CEGUI::RendererException",0x18);
  local_4 = 0;
  CEGUI__unknown_011c2520(this,param_1,local_28,param_2,param_3);
  if (7 < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24,uVar1);
  }
  *(undefined ***)this = CEGUI::RendererException::vftable;
  ExceptionList = local_c;
  return this;
}


/* [String discovery] Debug string: "u"CEGUI::TinyXMLParser - Official tinyXML based parser module
   for CEGUI""
   String at: 01b12028 */

undefined4 * __fastcall CEGUI_TinyXMLParser(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01790678;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0120ed20(param_1);
  local_4 = 0;
  *param_1 = CEGUI::TinyXMLParser::vftable;
  FUN_00438520(param_1 + 1,L"CEGUI::TinyXMLParser - Official tinyXML based parser module for CEGUI",
               0x45);
  ExceptionList = local_c;
  return param_1;
}


