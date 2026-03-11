/*
 * SGW.exe Decompilation - 10_game_systems
 * Classes: 12
 * Stargate Worlds Client
 */


/* ========== Action.c ========== */

/*
 * SGW.exe - Action (1 functions)
 */

undefined4 * __thiscall Action__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_016f91c8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = Action::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== ActionSequenceRef.c ========== */

/*
 * SGW.exe - ActionSequenceRef (1 functions)
 */

float10 __fastcall ActionSequenceRef__vfunc_0(int param_1)

{
  return (float10)*(float *)(param_1 + 0x28);
}




/* ========== BindableAction.c ========== */

/*
 * SGW.exe - BindableAction (1 functions)
 */

/* Also in vtable: BindableAction (slot 0) */

undefined4 * __thiscall BindableAction__vfunc_0(void *this,byte param_1)

{
  FUN_00955e10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Command.c ========== */

/*
 * SGW.exe - Command (1 functions)
 */

undefined4 * __thiscall Command__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017122c8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = Command::vftable;
  local_4 = 0xffffffff;
  FUN_00e4c570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== InteractionSet.c ========== */

/*
 * SGW.exe - InteractionSet (1 functions)
 */

undefined4 * __thiscall InteractionSet__vfunc_0(void *this,byte param_1)

{
  FUN_00d22c00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== LibCategoryBase.c ========== */

/*
 * SGW.exe - LibCategoryBase (1 functions)
 */

/* Also in vtable: LibCategoryBase (slot 0) */

undefined4 * __thiscall LibCategoryBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = LibCategoryBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== SystemOptionBase.c ========== */

/*
 * SGW.exe - SystemOptionBase (1 functions)
 */

/* Also in vtable: SystemOptionBase (slot 0) */

undefined4 * __thiscall SystemOptionBase__vfunc_0(void *this,byte param_1)

{
  FUN_0094eff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== SystemOptionPolicyBool.c ========== */

/*
 * SGW.exe - SystemOptionPolicyBool (6 functions)
 */

/* [VTable] SystemOptionPolicyBool virtual function #1
   VTable: vtable_SystemOptionPolicyBool at 0183ff24 */

void SystemOptionPolicyBool__vfunc_1(void)

{
  return;
}


/* [VTable] SystemOptionPolicyBool virtual function #2
   VTable: vtable_SystemOptionPolicyBool at 0183ff24 */

int SystemOptionPolicyBool__vfunc_2(int param_1)

{
  undefined1 *puVar1;
  
  puVar1 = *(undefined1 **)(param_1 + 0x14);
  if (puVar1 != (undefined1 *)0x0) {
    return CONCAT31((int3)((uint)puVar1 >> 8),*puVar1);
  }
  return 0;
}


/* [VTable] SystemOptionPolicyBool virtual function #3
   VTable: vtable_SystemOptionPolicyBool at 0183ff24 */

void SystemOptionPolicyBool__vfunc_3(int param_1,undefined1 *param_2)

{
  undefined1 *puVar1;
  
  puVar1 = CME_UIScreen__unknown_00a3b480(*(int *)(param_1 + 0x1c),1);
  *(undefined1 **)(param_1 + 0x14) = puVar1;
  *puVar1 = *param_2;
  return;
}


/* [VTable] SystemOptionPolicyBool virtual function #4
   VTable: vtable_SystemOptionPolicyBool at 0183ff24 */

undefined1 SystemOptionPolicyBool__vfunc_4(int param_1)

{
  return *(undefined1 *)(param_1 + 0x48);
}


/* [VTable] SystemOptionPolicyBool virtual function #5
   VTable: vtable_SystemOptionPolicyBool at 0183ff24 */

undefined4 SystemOptionPolicyBool__vfunc_5(int param_1)

{
  undefined1 *puVar1;
  
  puVar1 = *(undefined1 **)(param_1 + 0x50);
  if (puVar1 != (undefined1 *)0x0) {
    return CONCAT31((int3)((uint)puVar1 >> 8),*puVar1);
  }
  return (uint)*(byte *)(param_1 + 0x48);
}


/* Also in vtable: SystemOptionPolicyBool (slot 0) */

undefined4 * __thiscall SystemOptionPolicyBool__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = SystemOptionPolicyBool::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== SystemOptionPolicyEnum.c ========== */

/*
 * SGW.exe - SystemOptionPolicyEnum (8 functions)
 */

/* [VTable] SystemOptionPolicyEnum virtual function #2
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

undefined4 SystemOptionPolicyEnum__vfunc_2(int param_1)

{
  if (*(undefined4 **)(param_1 + 0xc) != (undefined4 *)0x0) {
    return **(undefined4 **)(param_1 + 0xc);
  }
  return 0;
}


/* [VTable] SystemOptionPolicyEnum virtual function #3
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

void SystemOptionPolicyEnum__vfunc_3(int param_1,undefined4 *param_2)

{
  undefined4 *puVar1;
  
  puVar1 = (undefined4 *)CME_UIScreen__unknown_00a3b480(*(int *)(param_1 + 0x1c),4);
  *(undefined4 **)(param_1 + 0xc) = puVar1;
  *puVar1 = *param_2;
  return;
}


/* [VTable] SystemOptionPolicyEnum virtual function #5
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

undefined4 __fastcall SystemOptionPolicyEnum__vfunc_5(int param_1)

{
  return *(undefined4 *)(param_1 + 4);
}


/* [VTable] SystemOptionPolicyEnum virtual function #4
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

undefined4 __fastcall SystemOptionPolicyEnum__vfunc_4(int param_1)

{
  return *(undefined4 *)(param_1 + 4);
}


/* [VTable] SystemOptionPolicyEnum virtual function #7
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

undefined4 __fastcall SystemOptionPolicyEnum__vfunc_7(int param_1)

{
  return *(undefined4 *)(param_1 + 8);
}


/* [VTable] SystemOptionPolicyEnum virtual function #6
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

undefined4 __fastcall SystemOptionPolicyEnum__vfunc_6(int param_1)

{
  return *(undefined4 *)(param_1 + 8);
}


/* WARNING: Removing unreachable block (ram,0x00576d1c) */
/* [VTable] SystemOptionPolicyEnum virtual function #1
   VTable: vtable_SystemOptionPolicyEnum at 01840060 */

void __thiscall SystemOptionPolicyEnum__vfunc_1(void *this,undefined4 *param_1,int *param_2)

{
  undefined4 *this_00;
  uint uVar1;
  undefined4 *local_1c;
  uint local_18;
  undefined4 local_14 [2];
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  this_00 = param_1;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0168f522;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  HVSystemOptionPolicyEnum__unknown_0093c0e0(param_1,&local_1c);
  local_4 = 0;
  uVar1 = *(uint *)((int)this + 0x10);
  if (*(uint *)((int)this + 0x14) < uVar1) {
    _invalid_parameter_noinfo();
  }
  local_18 = *(uint *)((int)this + 0x14);
  if (local_18 < *(uint *)((int)this + 0x10)) {
    _invalid_parameter_noinfo();
  }
  for (; uVar1 != local_18; uVar1 = uVar1 + 0x24) {
    HVSystemOptionPolicyEnum__unknown_0093c1b0(this_00,&param_1);
    local_4._0_1_ = 1;
    if (*(uint *)((int)this + 0x14) <= uVar1) {
      _invalid_parameter_noinfo();
    }
    HVSystemOptionPolicyEnum__unknown_00577b60(this_00,(int *)&param_1,L"text",uVar1);
    HVSystemOptionPolicyEnum__unknown_00577be0
              (this_00,(int *)&param_1,L"value",(int *)(uVar1 + 0x1c));
    HVSystemOptionPolicyEnum__unknown_0093b830(local_14,(lua_State *)*this_00);
    local_4._0_1_ = 2;
    HVSystemOptionPolicyEnum__unknown_0093b8a0((int)local_1c);
    FUN_0093bd90((int *)&param_1);
    FUN_0093bb70(this_00);
    local_4._0_1_ = 1;
    HVSystemOptionPolicyEnum__unknown_0093b870(local_14);
    local_4 = (uint)local_4._1_3_ << 8;
    if (((param_1 != (undefined4 *)0x0) && (param_1[1] = param_1[1] + -1, (int)param_1[1] < 1)) &&
       (param_1 != (undefined4 *)0x0)) {
      (**(code **)*param_1)(1);
    }
    if (*(uint *)((int)this + 0x14) <= uVar1) {
      _invalid_parameter_noinfo();
    }
  }
  HVSystemOptionPolicyEnum__unknown_00577a50(this_00,param_2,L"possibleValues",(int *)&local_1c);
  local_4 = 0xffffffff;
  if (((local_1c != (undefined4 *)0x0) && (local_1c[1] = local_1c[1] + -1, (int)local_1c[1] < 1)) &&
     (local_1c != (undefined4 *)0x0)) {
    (**(code **)*local_1c)(1);
  }
  ExceptionList = local_c;
  return;
}


/* Also in vtable: SystemOptionPolicyEnum (slot 0) */

undefined4 * __thiscall SystemOptionPolicyEnum__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0168f60b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = SystemOptionPolicyEnum::vftable;
  local_4 = 0xffffffff;
  FUN_005785b0((int)this + 0xc);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== SystemOptionPolicyFloat.c ========== */

/*
 * SGW.exe - SystemOptionPolicyFloat (6 functions)
 */

/* [VTable] SystemOptionPolicyFloat virtual function #2
   VTable: vtable_SystemOptionPolicyFloat at 0183ff40 */

float10 SystemOptionPolicyFloat__vfunc_2(int param_1)

{
  if (*(float **)(param_1 + 0x10) != (float *)0x0) {
    return (float10)**(float **)(param_1 + 0x10);
  }
  return (float10)0;
}


/* [VTable] SystemOptionPolicyFloat virtual function #3
   VTable: vtable_SystemOptionPolicyFloat at 0183ff40 */

void SystemOptionPolicyFloat__vfunc_3(int param_1,undefined4 *param_2)

{
  undefined4 uVar1;
  undefined4 *puVar2;
  
  puVar2 = (undefined4 *)CME_UIScreen__unknown_00a3b480(*(int *)(param_1 + 0x1c),4);
  uVar1 = *param_2;
  *(undefined4 **)(param_1 + 0x10) = puVar2;
  *puVar2 = uVar1;
  return;
}


/* [VTable] SystemOptionPolicyFloat virtual function #4
   VTable: vtable_SystemOptionPolicyFloat at 0183ff40 */

float10 SystemOptionPolicyFloat__vfunc_4(int param_1)

{
  return (float10)*(float *)(param_1 + 0x44);
}


/* [VTable] SystemOptionPolicyFloat virtual function #5
   VTable: vtable_SystemOptionPolicyFloat at 0183ff40 */

float10 SystemOptionPolicyFloat__vfunc_5(int param_1)

{
  if (*(float **)(param_1 + 0x4c) != (float *)0x0) {
    return (float10)**(float **)(param_1 + 0x4c);
  }
  return (float10)*(float *)(param_1 + 0x44);
}


/* [VTable] SystemOptionPolicyFloat virtual function #1
   VTable: vtable_SystemOptionPolicyFloat at 0183ff40 */

void __thiscall SystemOptionPolicyFloat__vfunc_1(void *this,void *param_1,int *param_2)

{
  FUN_00577ad0(param_1,param_2,
               (wchar_t *)
               (CME::Detail::PropertyNode::
                Property<class_CME::BasicPropertyTree<struct_TypeList::TypeList<unsigned_char,struct_TypeList::TypeList<char,struct_TypeList::TypeList<unsigned_short,struct_TypeList::TypeList<short,struct_TypeList::TypeList<unsigned_long,struct_TypeList::TypeList<long,struct_TypeList::TypeList<unsigned___int64,struct_TypeList::TypeList<__int64,struct_TypeList::TypeList<float,struct_TypeList::TypeList<class_std::basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_tbb::scalable_allocator<wchar_t>_>,struct_TypeList::TypeList<class_Vector2,struct_TypeList::TypeList<class_Vector3,struct_TypeList::TypeList<class_Vector4,struct_TypeList::NullType>_>_>_>_>_>_>_>_>_>_>_>_>_>_>
                ::vftable + 1),(float *)((int)this + 4));
  FUN_00577ad0(param_1,param_2,L"max",(float *)((int)this + 8));
  return;
}


/* Also in vtable: SystemOptionPolicyFloat (slot 0) */

undefined4 * __thiscall SystemOptionPolicyFloat__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = SystemOptionPolicyFloat::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== WorldMap.c ========== */

/*
 * SGW.exe - WorldMap (1 functions)
 */

undefined4 * __thiscall WorldMap__vfunc_0(void *this,byte param_1)

{
  FUN_00e58200(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== WorldSequenceRef.c ========== */

/*
 * SGW.exe - WorldSequenceRef (1 functions)
 */

float10 WorldSequenceRef__vfunc_0(void)

{
  return (float10)0;
}



