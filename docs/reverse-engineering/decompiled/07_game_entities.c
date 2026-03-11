/*
 * SGW.exe Decompilation - 07_game_entities
 * Classes: 28
 * Stargate Worlds Client
 */


/* ========== AlignmentEntry.c ========== */

/*
 * SGW.exe - AlignmentEntry (1 functions)
 */

void * AlignmentEntry__vfunc_0(void *param_1)

{
  int iVar1;
  void *pvVar2;
  void *pvVar3;
  uint uVar4;
  undefined1 *puVar5;
  wchar_t local_110 [4];
  undefined1 auStack_108 [4];
  undefined4 local_104 [4];
  undefined4 local_f4;
  uint local_f0;
  undefined1 auStack_ec [28];
  undefined1 auStack_d0 [28];
  undefined1 auStack_b4 [28];
  undefined1 auStack_98 [28];
  undefined1 auStack_7c [28];
  undefined1 auStack_60 [28];
  undefined1 auStack_44 [28];
  undefined1 auStack_28 [28];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01711036;
  pvStack_c = ExceptionList;
  local_110[2] = L'\0';
  local_110[3] = L'\0';
  local_f0 = 7;
  local_110[0] = L'\0';
  local_110[1] = L'\0';
  local_f4 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<wchar_t>::assign((wchar_t *)local_104,local_110);
  uStack_4 = 1;
  iVar1 = FUN_00c66ad0();
  iVar1 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar1 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GameBeing::RTTI_Type_Descriptor,0);
  if (iVar1 == 0) {
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    local_110[0] = L'\0';
    local_110[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_110);
    uVar4 = std::char_traits<wchar_t>::length(L"UNKNOWN PLAYER TYPE");
    FUN_00438520(param_1,L"UNKNOWN PLAYER TYPE",uVar4);
    local_110[2] = L'\x01';
    local_110[3] = L'\0';
    uStack_4 = uStack_4 & 0xffffff00;
    if (7 < local_f0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_104[0]);
    }
  }
  else {
    if (*(char *)(iVar1 + 0x135) == '\x01') {
      pvVar2 = FUN_00e9f810(auStack_28,iVar1);
      uStack_4._0_1_ = 5;
      pvVar3 = FUN_00ea01e0(auStack_60,(undefined4 *)0x66d9);
      uStack_4._0_1_ = 6;
      pvVar2 = FUN_0047b670(auStack_98,pvVar3,(int)pvVar2);
      uStack_4._0_1_ = 7;
      FUN_00437900(auStack_108,pvVar2,0,0xffffffff);
      uStack_4._0_1_ = 6;
      FUN_00433f40((int)auStack_98);
      uStack_4._0_1_ = 5;
      FUN_00433f40((int)auStack_60);
      puVar5 = auStack_28;
    }
    else if (*(char *)(iVar1 + 0x135) == '\x02') {
      pvVar2 = FUN_00e9f810(auStack_d0,iVar1);
      uStack_4._0_1_ = 2;
      pvVar3 = FUN_00ea01e0(auStack_7c,(undefined4 *)0x66d8);
      uStack_4._0_1_ = 3;
      pvVar2 = FUN_0047b670(auStack_ec,pvVar3,(int)pvVar2);
      uStack_4._0_1_ = 4;
      FUN_00437900(auStack_108,pvVar2,0,0xffffffff);
      uStack_4._0_1_ = 3;
      FUN_00433f40((int)auStack_ec);
      uStack_4._0_1_ = 2;
      FUN_00433f40((int)auStack_7c);
      puVar5 = auStack_d0;
    }
    else {
      pvVar2 = FUN_00e9f810(auStack_b4,iVar1);
      uStack_4._0_1_ = 8;
      pvVar2 = FUN_0093d8c0(auStack_44,L"UNKNOWN ALIGNMENT",(int)pvVar2);
      uStack_4._0_1_ = 9;
      FUN_00437900(auStack_108,pvVar2,0,0xffffffff);
      uStack_4._0_1_ = 8;
      FUN_00433f40((int)auStack_44);
      puVar5 = auStack_b4;
    }
    uStack_4._0_1_ = 1;
    FUN_00433f40((int)puVar5);
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    local_110[0] = L'\0';
    local_110[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_110);
    FUN_00437900(param_1,auStack_108,0,0xffffffff);
    local_110[2] = L'\x01';
    local_110[3] = L'\0';
    uStack_4 = (uint)uStack_4._1_3_ << 8;
    if (7 < local_f0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_104[0]);
    }
  }
  local_110[2] = L'\x01';
  local_110[3] = L'\0';
  local_f0 = 7;
  local_110[0] = L'\0';
  local_110[1] = L'\0';
  local_f4 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_104,local_110);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== ArchetypeEntry.c ========== */

/*
 * SGW.exe - ArchetypeEntry (1 functions)
 */

void * ArchetypeEntry__vfunc_0(void *param_1)

{
  int iVar1;
  void *pvVar2;
  void *pvVar3;
  uint uVar4;
  undefined4 uVar5;
  undefined4 *puVar6;
  wchar_t local_48 [2];
  undefined1 auStack_44 [4];
  undefined4 auStack_40 [4];
  undefined4 uStack_30;
  uint uStack_2c;
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_017110d9;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar1 = FUN_00c66ad0();
  iVar1 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar1 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GameBeing::RTTI_Type_Descriptor,0);
  if (iVar1 == 0) {
    uVar5 = 0xffffffff;
  }
  else {
    uVar5 = *(undefined4 *)(iVar1 + 0x138);
  }
  switch(uVar5) {
  case 1:
    puVar6 = (undefined4 *)0x66cb;
    break;
  case 2:
    puVar6 = (undefined4 *)0x66cc;
    break;
  case 3:
    puVar6 = (undefined4 *)0x66cd;
    break;
  case 4:
    puVar6 = (undefined4 *)0x66ce;
    break;
  case 5:
    puVar6 = (undefined4 *)0x66d2;
    break;
  case 6:
    puVar6 = (undefined4 *)0x66d1;
    break;
  case 7:
    puVar6 = (undefined4 *)0x66d0;
    break;
  case 8:
    puVar6 = (undefined4 *)0x66cf;
    break;
  default:
    pvVar3 = FUN_00e9f810(auStack_28,iVar1);
    iStack_4 = 3;
    uStack_2c = 7;
    local_48[0] = L'\0';
    local_48[1] = L'\0';
    uStack_30 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_40,local_48);
    uVar4 = std::char_traits<wchar_t>::length(L"UNKNOWN ARCHETYPE");
    FUN_00438520(auStack_44,L"UNKNOWN ARCHETYPE",uVar4);
    iStack_4._0_1_ = 4;
    FUN_0047b670(param_1,auStack_44,(int)pvVar3);
    iStack_4._0_1_ = 3;
    if (7 < uStack_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_40[0]);
    }
    uStack_2c = 7;
    local_48[0] = L'\0';
    local_48[1] = L'\0';
    uStack_30 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_40,local_48);
    iStack_4 = (uint)iStack_4._1_3_ << 8;
    if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_24[0]);
    }
    goto LAB_00ea2bf8;
  }
  pvVar3 = FUN_00e9f810(auStack_28,iVar1);
  iStack_4 = 1;
  pvVar2 = FUN_00ea01e0(auStack_44,puVar6);
  iStack_4._0_1_ = 2;
  FUN_0047b670(param_1,pvVar2,(int)pvVar3);
  iStack_4._0_1_ = 1;
  if (7 < uStack_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_40[0]);
  }
  uStack_2c = 7;
  local_48[0] = L'\0';
  local_48[1] = L'\0';
  uStack_30 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_40,local_48);
  iStack_4 = (uint)iStack_4._1_3_ << 8;
  if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
LAB_00ea2bf8:
  uStack_10 = 7;
  local_48[0] = L'\0';
  local_48[1] = L'\0';
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,local_48);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== FirstNameEntry.c ========== */

/*
 * SGW.exe - FirstNameEntry (1 functions)
 */

void * FirstNameEntry__vfunc_0(void *param_1)

{
  int iVar1;
  void *pvVar2;
  uint uVar3;
  wchar_t awStack_2c [2];
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01710e91;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar1 = FUN_00c66ad0();
  iVar1 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar1 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GameBeing::RTTI_Type_Descriptor,0);
  if (iVar1 == 0) {
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    awStack_2c[0] = L'\0';
    awStack_2c[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),awStack_2c);
    uVar3 = std::char_traits<wchar_t>::length(L"UNKNOWN PLAYER TYPE");
    FUN_00438520(param_1,L"UNKNOWN PLAYER TYPE",uVar3);
    ExceptionList = pvStack_c;
    return param_1;
  }
  pvVar2 = FUN_00e9f810(auStack_28,iVar1);
  uStack_4 = 1;
  FUN_0047b670(param_1,(void *)(iVar1 + 0x10c),(int)pvVar2);
  uStack_4 = uStack_4 & 0xffffff00;
  if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 7;
  awStack_2c[0] = L'\0';
  awStack_2c[1] = L'\0';
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,awStack_2c);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== FullNameEntry.c ========== */

/*
 * SGW.exe - FullNameEntry (1 functions)
 */

void * __fastcall FullNameEntry__vfunc_0(int param_1)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  void *pvVar4;
  void *pvVar5;
  void *pvVar6;
  wchar_t awStack_a0 [2];
  undefined4 local_9c;
  undefined1 auStack_98 [4];
  wchar_t awStack_94 [4];
  undefined4 uStack_8c;
  uint uStack_88;
  undefined4 uStack_84;
  undefined4 uStack_80;
  undefined1 auStack_7c [12];
  undefined4 uStack_70;
  uint uStack_6c;
  undefined1 auStack_68 [4];
  undefined4 uStack_64;
  undefined1 auStack_60 [12];
  undefined4 uStack_54;
  uint uStack_50;
  undefined4 uStack_48;
  undefined1 auStack_44 [12];
  undefined4 uStack_38;
  uint uStack_34;
  undefined4 uStack_2c;
  undefined1 auStack_28 [12];
  undefined4 uStack_1c;
  uint uStack_18;
  void *pvStack_14;
  void *pvStack_c;
  undefined1 *puStack_8;
  void *pvStack_4;
  
  pvStack_4 = (void *)0xffffffff;
  puStack_8 = &LAB_01710f35;
  pvStack_c = ExceptionList;
  local_9c = 0;
  ExceptionList = &pvStack_c;
  iVar1 = FUN_00c66ad0();
  iVar1 = *(int *)(iVar1 + 0x8c);
  iVar2 = FUN_00c66ad0();
  uVar3 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar2 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GameBeing::RTTI_Type_Descriptor,0);
  if (*(int *)(iVar1 + 0xac) == 0) {
    pvVar4 = FUN_00e9f810(auStack_60,uVar3);
    pvStack_4 = (void *)0x1;
    uStack_80 = 7;
    awStack_a0[0] = L'\0';
    awStack_a0[1] = L'\0';
    uStack_84 = 0;
    std::char_traits<wchar_t>::assign(awStack_94,awStack_a0);
    pvStack_4 = (void *)CONCAT31(pvStack_4._1_3_,2);
    pvVar5 = (void *)(*(code *)**(undefined4 **)(param_1 + 4))(auStack_7c,auStack_98);
    pvVar6 = pvStack_4;
    pvStack_c._0_1_ = 3;
    FUN_0047b670(pvStack_4,pvVar5,(int)pvVar4);
    pvStack_c._0_1_ = 2;
    if (7 < uStack_6c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_80);
    }
    uStack_6c = 7;
    uStack_70 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&uStack_80,(wchar_t *)&stack0xffffff58);
    pvStack_c._0_1_ = 1;
    if (7 < uStack_88) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_9c);
    }
    uStack_88 = 7;
    uStack_8c = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&local_9c,(wchar_t *)&stack0xffffff58);
    pvStack_c = (void *)((uint)pvStack_c._1_3_ << 8);
    if (7 < uStack_50) {
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_64);
    }
    uStack_50 = 7;
    uStack_54 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&uStack_64,(wchar_t *)&stack0xffffff58);
  }
  else {
    pvVar4 = FUN_00e9f810(auStack_28,uVar3);
    pvStack_4 = (void *)0x4;
    uStack_80 = 7;
    awStack_a0[0] = L'\0';
    awStack_a0[1] = L'\0';
    uStack_84 = 0;
    std::char_traits<wchar_t>::assign(awStack_94,awStack_a0);
    pvStack_4 = (void *)CONCAT31(pvStack_4._1_3_,5);
    iVar2 = (*(code *)**(undefined4 **)(param_1 + 4))(auStack_44,auStack_98);
    pvStack_c._0_1_ = 6;
    pvVar6 = FUN_0043a100(&uStack_84,(void *)(iVar1 + 0x98),L" ");
    pvStack_c._0_1_ = 7;
    pvVar5 = FUN_0047b670(auStack_68,pvVar6,iVar2);
    pvVar6 = pvStack_4;
    pvStack_c._0_1_ = 8;
    FUN_0047b670(pvStack_4,pvVar5,(int)pvVar4);
    pvStack_c._0_1_ = 7;
    if (7 < uStack_50) {
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_64);
    }
    uStack_50 = 7;
    uStack_54 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&uStack_64,(wchar_t *)&stack0xffffff58);
    pvStack_c._0_1_ = 6;
    if (7 < uStack_6c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_80);
    }
    uStack_6c = 7;
    uStack_70 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&uStack_80,(wchar_t *)&stack0xffffff58);
    pvStack_c._0_1_ = 5;
    if (7 < uStack_34) {
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_48);
    }
    uStack_34 = 7;
    uStack_38 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&uStack_48,(wchar_t *)&stack0xffffff58);
    pvStack_c._0_1_ = 4;
    if (7 < uStack_88) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_9c);
    }
    uStack_88 = 7;
    uStack_8c = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&local_9c,(wchar_t *)&stack0xffffff58);
    pvStack_c = (void *)((uint)pvStack_c._1_3_ << 8);
    if (7 < uStack_18) {
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_2c);
    }
    uStack_18 = 7;
    uStack_1c = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)&uStack_2c,(wchar_t *)&stack0xffffff58);
  }
  ExceptionList = pvStack_14;
  return pvVar6;
}




/* ========== GameAccount.c ========== */

/*
 * SGW.exe - GameAccount (1 functions)
 */

undefined4 * __thiscall GameAccount__vfunc_0(void *this,byte param_1)

{
  FUN_00e73fa0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GameBeing.c ========== */

/*
 * SGW.exe - GameBeing (1 functions)
 */

undefined4 * __thiscall GameBeing__vfunc_0(void *this,byte param_1)

{
  FUN_00e00ac0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GameEntity.c ========== */

/*
 * SGW.exe - GameEntity (3 functions)
 */

undefined4 * __thiscall GameEntity__vfunc_0(void *this,byte param_1)

{
  FUN_00e6e880(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* WARNING: Removing unreachable block (ram,0x00ebfce5) */
/* WARNING: Removing unreachable block (ram,0x00ebff1e) */
/* WARNING: Type propagation algorithm not settling */

void __thiscall GameEntity__unknown_00ebfca0(void *this,void *param_1)

{
  uint *puVar1;
  type_info *ptVar2;
  char *pcVar3;
  uint uVar4;
  undefined4 *puVar5;
  int *piVar6;
  basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> *pbVar7;
  undefined4 *puVar8;
  undefined **ppuVar9;
  void *pvVar10;
  code *pcVar11;
  uint *puVar12;
  wchar_t *in_stack_ffffff50;
  undefined **in_stack_ffffff5c;
  __type_info_node *in_stack_ffffff60;
  wchar_t *in_stack_ffffff68;
  wchar_t *in_stack_ffffff74;
  int iVar13;
  _func_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr
  *p_Var14;
  char cStack_69;
  void *pvStack_68;
  int iStack_64;
  int aiStack_60 [2];
  undefined1 auStack_58 [4];
  int iStack_54;
  int iStack_50;
  wchar_t *pwStack_4c;
  int aiStack_48 [2];
  char acStack_40 [12];
  undefined4 uStack_34;
  undefined4 uStack_30;
  uint uStack_2c;
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_01713148;
  pvStack_c = ExceptionList;
  puVar12 = *(uint **)((int)this + 0x1c);
  pvVar10 = this;
  ExceptionList = &pvStack_c;
  pvStack_68 = this;
  if (*(uint **)((int)this + 0x20) < puVar12) {
    ExceptionList = &pvStack_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(uint **)((int)this + 0x20);
    if (puVar1 < *(uint **)((int)this + 0x1c)) {
      _invalid_parameter_noinfo();
    }
    pcVar11 = _invalid_parameter_noinfo_exref;
    if (puVar12 == puVar1) break;
    if (*(uint **)((int)this + 0x20) <= puVar12) {
      _invalid_parameter_noinfo();
    }
    FUN_00f46b50((void *)((int)pvVar10 + 0x28),&iStack_54,puVar12);
    iStack_64 = 0;
    in_stack_ffffff68 =
         L"쒃㤘⑜༘鶅\x01謀\xf80d\xe218㠁༙疄\x01㬀ࡵٲᗿ隸žᚋꑨ\xf054刁\xefe8㝾茀ӄ좋ᗿﱌž\xf88b䒍ጤ赐⑌兀䓇堤\x0f"
    ;
    in_stack_ffffff74 = pwStack_4c;
    Mercury__unknown_00d09060(iStack_54,iStack_50,(int)pwStack_4c,aiStack_48[0],&iStack_64);
    if (iStack_64 == 0) {
      if (*PTR_DAT_01e218f8 != '\0') {
        if (*(uint **)((int)this + 0x20) <= puVar12) {
          _invalid_parameter_noinfo();
        }
        p_Var14 = (_func_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr
                   *)&DAT_01f054a4;
        iVar13 = 0xebfd6b;
        ptVar2 = (type_info *)__RTtypeid();
        pcVar3 = type_info::_name_internal_method(ptVar2,in_stack_ffffff60);
        pwStack_4c = (wchar_t *)0xf;
        in_stack_ffffff74 = (wchar_t *)((uint)in_stack_ffffff74 & 0xffffff);
        iStack_50 = 0;
        std::char_traits<char>::assign((char *)aiStack_60,&stack0xffffff77);
        uVar4 = std::char_traits<char>::length(pcVar3);
        FUN_004377d0(&iStack_64,pcVar3,uVar4);
        auStack_24[0] = 0;
        if (*(uint **)((int)this + 0x20) <= puVar12) {
          _invalid_parameter_noinfo();
        }
        puVar5 = FUN_00498ed0((void *)*puVar12,&stack0xffffff80);
        auStack_24[0]._0_1_ = 1;
        GameEntity__unknown_0048e8b0(aiStack_48,(int)&iStack_64);
        auStack_24[0] = CONCAT31(auStack_24[0]._1_3_,2);
        if (puVar5[1] == 0) {
          in_stack_ffffff5c = &PTR_017f8e10;
        }
        else {
          in_stack_ffffff5c = (undefined **)*puVar5;
        }
        in_stack_ffffff50 = L"    detaching stale existing component \'";
        piVar6 = (int *)GameEntity__unknown_00426050
                                  ((int *)(*(int *)(iVar13 + 4) + 8),
                                   L"    detaching stale existing component \'");
        piVar6 = (int *)Detail__unknown_00477ff0(piVar6,in_stack_ffffff5c);
        in_stack_ffffff60 = (__type_info_node *)0xebfe2a;
        piVar6 = (int *)GameEntity__unknown_00426050(piVar6,in_stack_ffffff68);
        pbVar7 = (basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> *)
                 GameEntity__unknown_00426050(piVar6,in_stack_ffffff74);
        std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<(pbVar7,p_Var14);
        iStack_4._0_1_ = 1;
        if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
          scalable_free();
        }
        uStack_10 = 7;
        iStack_64 = 0;
        uStack_14 = 0;
        std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&iStack_64);
        iStack_4 = (uint)iStack_4._1_3_ << 8;
        FUN_0041b420(aiStack_60);
        iStack_4 = 0xffffffff;
        if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
          scalable_free();
        }
        uStack_2c = 0xf;
        cStack_69 = '\0';
        uStack_30 = 0;
        std::char_traits<char>::assign(acStack_40,&cStack_69);
      }
      if (*(uint **)((int)this + 0x20) <= puVar12) {
        _invalid_parameter_noinfo();
      }
      FUN_0060fa40(param_1,(int *)*puVar12);
    }
    if (*(uint **)((int)this + 0x20) <= puVar12) {
      _invalid_parameter_noinfo();
    }
    puVar12 = puVar12 + 1;
    pvVar10 = pvStack_68;
  }
  puVar5 = *(undefined4 **)((int)pvVar10 + 0xc);
  if (*(undefined4 **)((int)pvVar10 + 0x10) < puVar5) {
    _invalid_parameter_noinfo();
  }
  do {
    puVar8 = *(undefined4 **)((int)pvVar10 + 0x10);
    if (puVar8 < *(undefined4 **)((int)pvVar10 + 0xc)) {
      (*pcVar11)();
    }
    if (puVar5 == puVar8) {
      ExceptionList = pvStack_c;
      return;
    }
    if (*PTR_DAT_01e218f8 != '\0') {
      if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
        (*pcVar11)();
      }
      p_Var14 = (_func_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr
                 *)&DAT_01f054a4;
      ptVar2 = (type_info *)__RTtypeid();
      pcVar3 = type_info::_name_internal_method(ptVar2,(__type_info_node *)in_stack_ffffff50);
      aiStack_60[1] = 0xf;
      aiStack_60[0] = 0;
      std::char_traits<char>::assign(&stack0xffffff90,&stack0xffffff67);
      uVar4 = std::char_traits<char>::length(pcVar3);
      FUN_004377d0(&stack0xffffff8c,pcVar3,uVar4);
      uStack_34 = 3;
      if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
        (*pcVar11)();
      }
      FUN_0049b190(puVar5 + 2,&stack0xffffff7c);
      uStack_34._0_1_ = 4;
      if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
        (*pcVar11)();
      }
      puVar8 = FUN_00498ed0((void *)puVar5[1],&stack0xffffff70);
      uStack_34._0_1_ = 5;
      GameEntity__unknown_0048e8b0(auStack_58,(int)&stack0xffffff8c);
      uStack_34 = CONCAT31(uStack_34._1_3_,6);
      if (puVar8[1] == 0) {
        ppuVar9 = &PTR_017f8e10;
      }
      else {
        ppuVar9 = (undefined **)*puVar8;
      }
      in_stack_ffffff50 = (wchar_t *)endl_exref;
      piVar6 = (int *)GameEntity__unknown_00426050
                                ((int *)(*(int *)(in_stack_ffffff68 + 2) + 8),
                                 L"    attaching new component \'");
      piVar6 = (int *)Detail__unknown_00477ff0(piVar6,ppuVar9);
      piVar6 = (int *)GameEntity__unknown_00426050(piVar6,in_stack_ffffff50);
      piVar6 = (int *)GameEntity__unknown_00426050(piVar6,(wchar_t *)in_stack_ffffff5c);
      piVar6 = (int *)GameEntity__unknown_00426050(piVar6,in_stack_ffffff68);
      pbVar7 = (basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> *)
               GameEntity__unknown_00426050(piVar6,in_stack_ffffff74);
      std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<(pbVar7,p_Var14);
      iStack_4._0_1_ = 5;
      if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_10 = 7;
      iStack_64 = 0;
      uStack_14 = 0;
      std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&iStack_64);
      iStack_4._0_1_ = 4;
      FUN_0041b420(aiStack_60);
      iStack_4 = CONCAT31(iStack_4._1_3_,3);
      FUN_0041b420(&iStack_54);
      iStack_4 = 0xffffffff;
      if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_2c = 0xf;
      cStack_69 = '\0';
      uStack_30 = 0;
      std::char_traits<char>::assign(acStack_40,&cStack_69);
      pcVar11 = _invalid_parameter_noinfo_exref;
    }
    if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
      (*pcVar11)();
    }
    *(uint *)(puVar5[1] + 0x50) = *(uint *)(puVar5[1] + 0x50) | 4;
    if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
      (*pcVar11)();
    }
    in_stack_ffffff74 = (wchar_t *)0xec0136;
    (**(code **)(*(int *)*puVar5 + 8))();
    if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
      (*pcVar11)();
    }
    puVar5 = puVar5 + 4;
  } while( true );
}


/* WARNING: Removing unreachable block (ram,0x00ec01a5) */
/* WARNING: Removing unreachable block (ram,0x00ec03de) */
/* WARNING: Type propagation algorithm not settling */

void __thiscall GameEntity__unknown_00ec0160(void *this,void *param_1)

{
  uint *puVar1;
  type_info *ptVar2;
  char *pcVar3;
  uint uVar4;
  undefined4 *puVar5;
  int *piVar6;
  basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> *pbVar7;
  undefined4 *puVar8;
  undefined **ppuVar9;
  void *pvVar10;
  code *pcVar11;
  uint *puVar12;
  wchar_t *in_stack_ffffff50;
  undefined **in_stack_ffffff5c;
  __type_info_node *in_stack_ffffff60;
  wchar_t *in_stack_ffffff68;
  wchar_t *in_stack_ffffff74;
  int iVar13;
  _func_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr
  *p_Var14;
  char cStack_69;
  void *pvStack_68;
  int iStack_64;
  int aiStack_60 [2];
  undefined1 auStack_58 [4];
  int iStack_54;
  int iStack_50;
  wchar_t *pwStack_4c;
  int aiStack_48 [2];
  char acStack_40 [12];
  undefined4 uStack_34;
  undefined4 uStack_30;
  uint uStack_2c;
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_01713148;
  pvStack_c = ExceptionList;
  puVar12 = *(uint **)((int)this + 0x1c);
  pvVar10 = this;
  ExceptionList = &pvStack_c;
  pvStack_68 = this;
  if (*(uint **)((int)this + 0x20) < puVar12) {
    ExceptionList = &pvStack_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(uint **)((int)this + 0x20);
    if (puVar1 < *(uint **)((int)this + 0x1c)) {
      _invalid_parameter_noinfo();
    }
    pcVar11 = _invalid_parameter_noinfo_exref;
    if (puVar12 == puVar1) break;
    if (*(uint **)((int)this + 0x20) <= puVar12) {
      _invalid_parameter_noinfo();
    }
    FUN_00f46b50((void *)((int)pvVar10 + 0x28),&iStack_54,puVar12);
    iStack_64 = 0;
    in_stack_ffffff68 =
         L"쒃㤘⑜༘鶅\x01謀\xf80d\xe218㠁༙疄\x01㬀ࡵٲᗿ隸žᚋꑨ\xf054刁⿨㝺茀ӄ좋ᗿﱌž\xf88b䒍ጤ赐⑌兀䓇堤\x0f"
    ;
    in_stack_ffffff74 = pwStack_4c;
    Mercury__unknown_00d09060(iStack_54,iStack_50,(int)pwStack_4c,aiStack_48[0],&iStack_64);
    if (iStack_64 == 0) {
      if (*PTR_DAT_01e218f8 != '\0') {
        if (*(uint **)((int)this + 0x20) <= puVar12) {
          _invalid_parameter_noinfo();
        }
        p_Var14 = (_func_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr
                   *)&DAT_01f054a4;
        iVar13 = 0xec022b;
        ptVar2 = (type_info *)__RTtypeid();
        pcVar3 = type_info::_name_internal_method(ptVar2,in_stack_ffffff60);
        pwStack_4c = (wchar_t *)0xf;
        in_stack_ffffff74 = (wchar_t *)((uint)in_stack_ffffff74 & 0xffffff);
        iStack_50 = 0;
        std::char_traits<char>::assign((char *)aiStack_60,&stack0xffffff77);
        uVar4 = std::char_traits<char>::length(pcVar3);
        FUN_004377d0(&iStack_64,pcVar3,uVar4);
        auStack_24[0] = 0;
        if (*(uint **)((int)this + 0x20) <= puVar12) {
          _invalid_parameter_noinfo();
        }
        puVar5 = FUN_00498ed0((void *)*puVar12,&stack0xffffff80);
        auStack_24[0]._0_1_ = 1;
        GameEntity__unknown_0048e8b0(aiStack_48,(int)&iStack_64);
        auStack_24[0] = CONCAT31(auStack_24[0]._1_3_,2);
        if (puVar5[1] == 0) {
          in_stack_ffffff5c = &PTR_017f8e10;
        }
        else {
          in_stack_ffffff5c = (undefined **)*puVar5;
        }
        in_stack_ffffff50 = L"    detaching stale existing component \'";
        piVar6 = (int *)GameEntity__unknown_00426050
                                  ((int *)(*(int *)(iVar13 + 4) + 8),
                                   L"    detaching stale existing component \'");
        piVar6 = (int *)Detail__unknown_00477ff0(piVar6,in_stack_ffffff5c);
        in_stack_ffffff60 = (__type_info_node *)0xec02ea;
        piVar6 = (int *)GameEntity__unknown_00426050(piVar6,in_stack_ffffff68);
        pbVar7 = (basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> *)
                 GameEntity__unknown_00426050(piVar6,in_stack_ffffff74);
        std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<(pbVar7,p_Var14);
        iStack_4._0_1_ = 1;
        if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
          scalable_free();
        }
        uStack_10 = 7;
        iStack_64 = 0;
        uStack_14 = 0;
        std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&iStack_64);
        iStack_4 = (uint)iStack_4._1_3_ << 8;
        FUN_0041b420(aiStack_60);
        iStack_4 = 0xffffffff;
        if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
          scalable_free();
        }
        uStack_2c = 0xf;
        cStack_69 = '\0';
        uStack_30 = 0;
        std::char_traits<char>::assign(acStack_40,&cStack_69);
      }
      if (*(uint **)((int)this + 0x20) <= puVar12) {
        _invalid_parameter_noinfo();
      }
      AKActorSpawnable__unknown_006e6cf0(param_1,(int *)*puVar12);
    }
    if (*(uint **)((int)this + 0x20) <= puVar12) {
      _invalid_parameter_noinfo();
    }
    puVar12 = puVar12 + 1;
    pvVar10 = pvStack_68;
  }
  puVar5 = *(undefined4 **)((int)pvVar10 + 0xc);
  if (*(undefined4 **)((int)pvVar10 + 0x10) < puVar5) {
    _invalid_parameter_noinfo();
  }
  do {
    puVar8 = *(undefined4 **)((int)pvVar10 + 0x10);
    if (puVar8 < *(undefined4 **)((int)pvVar10 + 0xc)) {
      (*pcVar11)();
    }
    if (puVar5 == puVar8) {
      ExceptionList = pvStack_c;
      return;
    }
    if (*PTR_DAT_01e218f8 != '\0') {
      if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
        (*pcVar11)();
      }
      p_Var14 = (_func_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr_basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>_ptr
                 *)&DAT_01f054a4;
      ptVar2 = (type_info *)__RTtypeid();
      pcVar3 = type_info::_name_internal_method(ptVar2,(__type_info_node *)in_stack_ffffff50);
      aiStack_60[1] = 0xf;
      aiStack_60[0] = 0;
      std::char_traits<char>::assign(&stack0xffffff90,&stack0xffffff67);
      uVar4 = std::char_traits<char>::length(pcVar3);
      FUN_004377d0(&stack0xffffff8c,pcVar3,uVar4);
      uStack_34 = 3;
      if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
        (*pcVar11)();
      }
      FUN_0049b190(puVar5 + 2,&stack0xffffff7c);
      uStack_34._0_1_ = 4;
      if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
        (*pcVar11)();
      }
      puVar8 = FUN_00498ed0((void *)puVar5[1],&stack0xffffff70);
      uStack_34._0_1_ = 5;
      GameEntity__unknown_0048e8b0(auStack_58,(int)&stack0xffffff8c);
      uStack_34 = CONCAT31(uStack_34._1_3_,6);
      if (puVar8[1] == 0) {
        ppuVar9 = &PTR_017f8e10;
      }
      else {
        ppuVar9 = (undefined **)*puVar8;
      }
      in_stack_ffffff50 = (wchar_t *)endl_exref;
      piVar6 = (int *)GameEntity__unknown_00426050
                                ((int *)(*(int *)(in_stack_ffffff68 + 2) + 8),
                                 L"    attaching new component \'");
      piVar6 = (int *)Detail__unknown_00477ff0(piVar6,ppuVar9);
      piVar6 = (int *)GameEntity__unknown_00426050(piVar6,in_stack_ffffff50);
      piVar6 = (int *)GameEntity__unknown_00426050(piVar6,(wchar_t *)in_stack_ffffff5c);
      piVar6 = (int *)GameEntity__unknown_00426050(piVar6,in_stack_ffffff68);
      pbVar7 = (basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> *)
               GameEntity__unknown_00426050(piVar6,in_stack_ffffff74);
      std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<(pbVar7,p_Var14);
      iStack_4._0_1_ = 5;
      if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_10 = 7;
      iStack_64 = 0;
      uStack_14 = 0;
      std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&iStack_64);
      iStack_4._0_1_ = 4;
      FUN_0041b420(aiStack_60);
      iStack_4 = CONCAT31(iStack_4._1_3_,3);
      FUN_0041b420(&iStack_54);
      iStack_4 = 0xffffffff;
      if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_2c = 0xf;
      cStack_69 = '\0';
      uStack_30 = 0;
      std::char_traits<char>::assign(acStack_40,&cStack_69);
      pcVar11 = _invalid_parameter_noinfo_exref;
    }
    if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
      (*pcVar11)();
    }
    *(uint *)(puVar5[1] + 0x50) = *(uint *)(puVar5[1] + 0x50) | 4;
    if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
      (*pcVar11)();
    }
    (**(code **)(*(int *)*puVar5 + 4))();
    if (*(undefined4 **)((int)pvVar10 + 0x10) <= puVar5) {
      (*pcVar11)();
    }
    puVar5 = puVar5 + 4;
  } while( true );
}




/* ========== GameEntityBase.c ========== */

/*
 * SGW.exe - GameEntityBase (1 functions)
 */

void __fastcall GameEntityBase__vfunc_0(undefined4 *param_1)

{
  *param_1 = GameEntityBase::vftable;
                    /* WARNING: Subroutine does not return */
  scalable_free(param_1[1]);
}




/* ========== GameEntityFactory_VGameAccount.c ========== */

/*
 * SGW.exe - GameEntityFactory_VGameAccount (1 functions)
 */

void GameEntityFactory_VGameAccount___EntityRegister__vfunc_0(void)

{
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)scalable_malloc(0x80);
  if (puVar1 == (undefined4 *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00e73d40(puVar1);
  ExceptionList = local_c;
  return;
}




/* ========== GameEntityFactory_VGameBeing.c ========== */

/*
 * SGW.exe - GameEntityFactory_VGameBeing (1 functions)
 */

void GameEntityFactory_VGameBeing___EntityRegister__vfunc_0(void)

{
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)scalable_malloc(0x16c);
  if (puVar1 == (undefined4 *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00e00830(puVar1);
  ExceptionList = local_c;
  return;
}




/* ========== GameEntityFactory_VGameEntity.c ========== */

/*
 * SGW.exe - GameEntityFactory_VGameEntity (1 functions)
 */

void GameEntityFactory_VGameEntity___EntityRegister__vfunc_0(void)

{
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)scalable_malloc(0xfc);
  if (puVar1 == (undefined4 *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00e6e760(puVar1);
  ExceptionList = local_c;
  return;
}




/* ========== GameEntityFactory_VGameMob.c ========== */

/*
 * SGW.exe - GameEntityFactory_VGameMob (1 functions)
 */

void GameEntityFactory_VGameMob___EntityRegister__vfunc_0(void)

{
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)scalable_malloc(0x170);
  if (puVar1 == (undefined4 *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00d316f0(puVar1);
  ExceptionList = local_c;
  return;
}




/* ========== GameEntityFactory_VGamePet.c ========== */

/*
 * SGW.exe - GameEntityFactory_VGamePet (1 functions)
 */

void GameEntityFactory_VGamePet___EntityRegister__vfunc_0(void)

{
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)scalable_malloc(0x1b4);
  if (puVar1 == (undefined4 *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00d39cb0(puVar1);
  ExceptionList = local_c;
  return;
}




/* ========== GameEntityFactory_VGamePlayer.c ========== */

/*
 * SGW.exe - GameEntityFactory_VGamePlayer (1 functions)
 */

void GameEntityFactory_VGamePlayer___EntityRegister__vfunc_0(void)

{
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)scalable_malloc(0x188);
  if (puVar1 == (undefined4 *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00e07140(puVar1);
  ExceptionList = local_c;
  return;
}




/* ========== GameMob.c ========== */

/*
 * SGW.exe - GameMob (1 functions)
 */

undefined4 * __thiscall GameMob__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016fd5b8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = GameMob::vftable;
  local_4 = 0xffffffff;
  FUN_00e00ac0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== GamePet.c ========== */

/*
 * SGW.exe - GamePet (1 functions)
 */

undefined4 * __thiscall GamePet__vfunc_0(void *this,byte param_1)

{
  FUN_00d3cc60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GamePlayer.c ========== */

/*
 * SGW.exe - GamePlayer (1 functions)
 */

undefined4 * __thiscall GamePlayer__vfunc_0(void *this,byte param_1)

{
  FUN_00e070d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GameWorldConstants.c ========== */

/*
 * SGW.exe - GameWorldConstants (1 functions)
 */

undefined4 * __thiscall GameWorldConstants__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = GameWorldConstants::vftable;
  if (DAT_01ef2270 != (undefined4 *)0x0) {
    (**(code **)*DAT_01ef2270)(1);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GenderEntry.c ========== */

/*
 * SGW.exe - GenderEntry (1 functions)
 */

void * __thiscall GenderEntry__vfunc_0(void *this,void *param_1,void *param_2)

{
  undefined4 uVar1;
  void *unaff_ESI;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d9379;
  local_c = ExceptionList;
  if (*(int *)((int)param_2 + 0x14) == 6) {
    ExceptionList = &local_c;
    (*(code *)**(undefined4 **)((int)this + 0x10))(param_1,param_2);
    ExceptionList = unaff_ESI;
    return param_1;
  }
  ExceptionList = &local_c;
  uVar1 = FUN_00e9f480(param_2,L" male=");
  *(undefined4 *)((int)this + 4) = uVar1;
  uVar1 = FUN_00e9f480(param_2,L" female=");
  *(undefined4 *)((int)this + 8) = uVar1;
  uVar1 = FUN_00e9f480(param_2,L" neutral=");
  *(undefined4 *)((int)this + 0xc) = uVar1;
  GenderedEntry__vfunc_0(this,param_1);
  ExceptionList = local_c;
  return param_1;
}




/* ========== GenderedEntry.c ========== */

/*
 * SGW.exe - GenderedEntry (1 functions)
 */

void * __thiscall GenderedEntry__vfunc_0(void *this,void *param_1)

{
  int iVar1;
  int iVar2;
  void *pvVar3;
  void *pvVar4;
  uint uVar5;
  undefined1 *puVar6;
  undefined4 *puVar7;
  wchar_t awStack_48 [2];
  undefined1 auStack_44 [28];
  undefined1 auStack_28 [28];
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_01711089;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar2 = FUN_00c66ad0();
  iVar2 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar2 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GamePlayer::RTTI_Type_Descriptor,0)
  ;
  if (iVar2 == 0) {
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    awStack_48[0] = L'\0';
    awStack_48[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),awStack_48);
    uVar5 = std::char_traits<wchar_t>::length(L"ProxyPlayer is not a GamePlayer!");
    FUN_00438520(param_1,L"ProxyPlayer is not a GamePlayer!",uVar5);
    ExceptionList = pvStack_c;
    return param_1;
  }
  iVar1 = *(int *)(iVar2 + 0x184);
  if (iVar1 == 1) {
    puVar7 = *(undefined4 **)((int)this + 4);
  }
  else if (iVar1 == 2) {
    puVar7 = *(undefined4 **)((int)this + 8);
  }
  else {
    if (iVar1 != 3) {
      pvVar3 = FUN_00e9f810(auStack_28,iVar2);
      iStack_4 = 1;
      pvVar4 = FUN_004398f0(auStack_44,L"UNKNOWN GENDER");
      iStack_4._0_1_ = 2;
      FUN_0047b670(param_1,pvVar4,(int)pvVar3);
      iStack_4._0_1_ = 1;
      FUN_00433f40((int)auStack_44);
      puVar6 = auStack_28;
      goto LAB_00ea2896;
    }
    puVar7 = *(undefined4 **)((int)this + 0xc);
  }
  if (puVar7 == (undefined4 *)0xffffffff) {
    pvVar3 = FUN_00e9f810(auStack_44,iVar2);
    iStack_4 = 3;
    pvVar4 = FUN_004398f0(auStack_28,(wchar_t *)&PTR_017f8e10);
    iStack_4._0_1_ = 4;
    FUN_0047b670(param_1,pvVar4,(int)pvVar3);
    iStack_4._0_1_ = 3;
  }
  else {
    pvVar3 = FUN_00e9f810(auStack_44,iVar2);
    iStack_4 = 5;
    pvVar4 = FUN_00ea01e0(auStack_28,puVar7);
    iStack_4._0_1_ = 6;
    FUN_0047b670(param_1,pvVar4,(int)pvVar3);
    iStack_4._0_1_ = 5;
  }
  FUN_00433f40((int)auStack_28);
  puVar6 = auStack_44;
LAB_00ea2896:
  iStack_4 = (uint)iStack_4._1_3_ << 8;
  FUN_00433f40((int)puVar6);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== KeyBindingEntry.c ========== */

/*
 * SGW.exe - KeyBindingEntry (1 functions)
 */

void * KeyBindingEntry__vfunc_0(void *param_1,void *param_2)

{
  uint uVar1;
  undefined *this;
  void *pvVar2;
  wchar_t ***pppwVar3;
  void **ppvVar4;
  undefined1 *puVar5;
  wchar_t local_50 [4];
  undefined4 local_48;
  wchar_t awStack_44 [2];
  wchar_t **appwStack_40 [4];
  int iStack_30;
  uint uStack_2c;
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_01711674;
  pvStack_c = ExceptionList;
  local_4 = 0;
  local_48 = 0;
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 7;
  local_50[0] = L'\0';
  local_50[1] = L'\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_50);
  local_4 = 0;
  local_48 = 1;
  FUN_00e9f5e0(awStack_44,param_2,L"key ");
  local_4 = 1;
  if (iStack_30 != 0) {
    pppwVar3 = (wchar_t ***)appwStack_40[0];
    if (uStack_2c < 8) {
      pppwVar3 = appwStack_40;
    }
    uStack_10 = 7;
    local_50[0] = L'\0';
    local_50[1] = L'\0';
    uStack_14 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,local_50);
    uVar1 = std::char_traits<wchar_t>::length((wchar_t *)pppwVar3);
    FUN_00438520(auStack_28,(wchar_t *)pppwVar3,uVar1);
    local_4._0_1_ = 2;
    puVar5 = auStack_28;
    ppvVar4 = &param_2;
    this = NVSystemOptionPolicyBool__unknown_0057d070();
    FUN_0057bc20(this,(uint *)ppvVar4,puVar5);
    local_4._0_1_ = 4;
    if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_24[0]);
    }
    uStack_10 = 7;
    local_50[0] = L'\0';
    local_50[1] = L'\0';
    uStack_14 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,local_50);
    if (param_2 == (void *)0x0) {
      pvVar2 = FUN_0093d8c0(auStack_28,L"Unknown action:",(int)awStack_44);
      local_4._0_1_ = 7;
      FUN_00437900(param_1,pvVar2,0,0xffffffff);
      local_4 = CONCAT31(local_4._1_3_,4);
      FUN_00433f40((int)auStack_28);
    }
    else {
      FUN_009559f0(param_2,(undefined4 *)local_50,0);
      local_4._0_1_ = 5;
      pvVar2 = FUN_009514f0(local_50,auStack_28);
      local_4._0_1_ = 6;
      FUN_00437900(param_1,pvVar2,0,0xffffffff);
      local_4._0_1_ = 5;
      FUN_00433f40((int)auStack_28);
      local_4 = CONCAT31(local_4._1_3_,4);
      FUN_00951360();
    }
    local_4 = CONCAT31(local_4._1_3_,1);
    NVSystemOptionPolicyBool__unknown_0057db50((uint *)&param_2);
  }
  local_4 = local_4 & 0xffffff00;
  if (7 < uStack_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(appwStack_40[0]);
  }
  uStack_2c = 7;
  param_2 = (void *)0x0;
  iStack_30 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)appwStack_40,(wchar_t *)&param_2);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== LiteralEntry.c ========== */

/*
 * SGW.exe - LiteralEntry (1 functions)
 */

void * __thiscall LiteralEntry__vfunc_0(void *this,void *param_1)

{
  wchar_t local_14 [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d9379;
  pvStack_c = ExceptionList;
  local_14[2] = L'\0';
  local_14[3] = L'\0';
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 7;
  local_14[0] = L'\0';
  local_14[1] = L'\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_14);
  FUN_00437900(param_1,(void *)((int)this + 4),0,0xffffffff);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== RaceEntry.c ========== */

/*
 * SGW.exe - RaceEntry (1 functions)
 */

void * RaceEntry__vfunc_0(void *param_1)

{
  int iVar1;
  void *pvVar2;
  void *pvVar3;
  uint uVar4;
  undefined4 uVar5;
  undefined4 *puVar6;
  wchar_t local_48 [2];
  undefined1 auStack_44 [4];
  undefined4 auStack_40 [4];
  undefined4 uStack_30;
  uint uStack_2c;
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_017110d9;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar1 = FUN_00c66ad0();
  iVar1 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar1 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GameBeing::RTTI_Type_Descriptor,0);
  if (iVar1 == 0) {
    uVar5 = 0xffffffff;
  }
  else {
    uVar5 = *(undefined4 *)(iVar1 + 0x138);
  }
  switch(uVar5) {
  case 1:
  case 2:
  case 3:
  case 4:
    puVar6 = (undefined4 *)0x66d3;
    break;
  case 5:
    puVar6 = (undefined4 *)0x66d6;
    break;
  case 6:
    puVar6 = (undefined4 *)0x66d5;
    break;
  case 7:
  case 8:
    puVar6 = (undefined4 *)0x66d4;
    break;
  default:
    pvVar3 = FUN_00e9f810(auStack_28,iVar1);
    iStack_4 = 3;
    uStack_2c = 7;
    local_48[0] = L'\0';
    local_48[1] = L'\0';
    uStack_30 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_40,local_48);
    uVar4 = std::char_traits<wchar_t>::length(L"UNKNOWN RACE");
    FUN_00438520(auStack_44,L"UNKNOWN RACE",uVar4);
    iStack_4._0_1_ = 4;
    FUN_0047b670(param_1,auStack_44,(int)pvVar3);
    iStack_4._0_1_ = 3;
    if (7 < uStack_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_40[0]);
    }
    uStack_2c = 7;
    local_48[0] = L'\0';
    local_48[1] = L'\0';
    uStack_30 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_40,local_48);
    iStack_4 = (uint)iStack_4._1_3_ << 8;
    if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_24[0]);
    }
    goto LAB_00ea2e4c;
  }
  pvVar3 = FUN_00e9f810(auStack_28,iVar1);
  iStack_4 = 1;
  pvVar2 = FUN_00ea01e0(auStack_44,puVar6);
  iStack_4._0_1_ = 2;
  FUN_0047b670(param_1,pvVar2,(int)pvVar3);
  iStack_4._0_1_ = 1;
  if (7 < uStack_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_40[0]);
  }
  uStack_2c = 7;
  local_48[0] = L'\0';
  local_48[1] = L'\0';
  uStack_30 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_40,local_48);
  iStack_4 = (uint)iStack_4._1_3_ << 8;
  if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
LAB_00ea2e4c:
  uStack_10 = 7;
  local_48[0] = L'\0';
  local_48[1] = L'\0';
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,local_48);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== RaceStringEntry.c ========== */

/*
 * SGW.exe - RaceStringEntry (1 functions)
 */

void * RaceStringEntry__vfunc_0(void *param_1,void *param_2)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 *puVar3;
  undefined4 *puVar4;
  undefined4 *puVar5;
  undefined4 uVar6;
  wchar_t *pwVar7;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01711109;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar1 = FUN_00c66ad0();
  iVar1 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar1 + 0x8c) + 4),0,
                          &GameEntityBase::RTTI_Type_Descriptor,&GameBeing::RTTI_Type_Descriptor,0);
  puVar2 = (undefined4 *)FUN_00e9f480(param_2,L" human=");
  puVar3 = (undefined4 *)FUN_00e9f480(param_2,L" jaffa=");
  puVar4 = (undefined4 *)FUN_00e9f480(param_2,L" asgard=");
  puVar5 = (undefined4 *)FUN_00e9f480(param_2,L" goauld=");
  if (iVar1 == 0) {
    uVar6 = 0xffffffff;
  }
  else {
    uVar6 = *(undefined4 *)(iVar1 + 0x138);
  }
  switch(uVar6) {
  case 1:
  case 2:
  case 3:
  case 4:
    puVar5 = puVar2;
    break;
  case 5:
    puVar5 = puVar4;
    break;
  case 6:
    break;
  case 7:
  case 8:
    puVar5 = puVar3;
    break;
  default:
    goto switchD_00ea2f4d_default;
  }
  if (-1 < (int)puVar5) {
    FUN_00ea01e0(param_1,puVar5);
    ExceptionList = pvStack_c;
    return param_1;
  }
  if (puVar5 == (undefined4 *)0xffffffff) {
    pwVar7 = (wchar_t *)&PTR_017f8e10;
  }
  else {
switchD_00ea2f4d_default:
    pwVar7 = L"UNKNOWN RACE";
  }
  FUN_004398f0(param_1,pwVar7);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== SettableEntry.c ========== */

/*
 * SGW.exe - SettableEntry (1 functions)
 */

void * SettableEntry__vfunc_0(void *param_1)

{
  uint uVar1;
  wchar_t local_14 [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d9379;
  pvStack_c = ExceptionList;
  local_14[2] = L'\0';
  local_14[3] = L'\0';
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 7;
  local_14[0] = L'\0';
  local_14[1] = L'\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_14);
  uVar1 = std::char_traits<wchar_t>::length(L"UNKNOWN TOKEN");
  FUN_00438520(param_1,L"UNKNOWN TOKEN",uVar1);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== StringInfo.c ========== */

/*
 * SGW.exe - StringInfo (1 functions)
 */

void * __thiscall StringInfo__vfunc_0(void *this,void *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0171142d;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_00ea5140(this,param_1);
  ExceptionList = local_c;
  return param_1;
}




/* ========== TaskCountEntry.c ========== */

/*
 * SGW.exe - TaskCountEntry (1 functions)
 */

void * TaskCountEntry__vfunc_0(void *param_1,void *param_2)

{
  int iVar1;
  uint uVar2;
  wchar_t local_90 [2];
  basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> local_8c [76];
  basic_ios<wchar_t,struct_std::char_traits<wchar_t>_> abStack_40 [52];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01710f6a;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_00e9f480(param_2,L" taskid=");
  iVar1 = FUN_00c66ad0();
  iVar1 = FUN_00d16790(*(int *)(*(int *)(iVar1 + 0x8c) + 0x20));
  if ((iVar1 == 0) || (iVar1 = *(int *)(iVar1 + 0xc), iVar1 < 0)) {
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    local_90[0] = L'\0';
    local_90[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_90);
    uVar2 = std::char_traits<wchar_t>::length(L"UNKNOWN TASK ID");
    FUN_00438520(param_1,L"UNKNOWN TASK ID",uVar2);
  }
  else {
    Detail__unknown_0042e9c0(local_8c,2,1);
    local_4 = 1;
    std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<(local_8c,iVar1);
    Detail__unknown_00439cb0(local_8c,param_1);
    local_4 = local_4 & 0xffffff00;
    FUN_0042ea60((int)abStack_40);
    std::basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>::
    ~basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>(abStack_40);
  }
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== TaskCountStringEntry.c ========== */

/*
 * SGW.exe - TaskCountStringEntry (1 functions)
 */

void * TaskCountStringEntry__vfunc_0(void *param_1,void *param_2)

{
  undefined4 *puVar1;
  undefined4 *puVar2;
  undefined4 *puVar3;
  int iVar4;
  bool bVar5;
  undefined **ppuVar6;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d9379;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = (undefined4 *)FUN_00e9f480(param_2,L" single=");
  puVar2 = (undefined4 *)FUN_00e9f480(param_2,L" plural=");
  puVar3 = (undefined4 *)FUN_00e9f480(param_2,L" zero=");
  FUN_00e9f480(param_2,L" taskid=");
  iVar4 = FUN_00c66ad0();
  iVar4 = FUN_00d16790(*(int *)(*(int *)(iVar4 + 0x8c) + 0x20));
  if (iVar4 == 0) {
LAB_00ea3118:
    ppuVar6 = (undefined **)L"UNKNOWN TASK ID";
  }
  else {
    iVar4 = *(int *)(iVar4 + 0xc);
    if (iVar4 == 1) {
      if (-1 < (int)puVar1) {
        FUN_00ea01e0(param_1,puVar1);
        ExceptionList = local_c;
        return param_1;
      }
    }
    else {
      if (iVar4 < 1) {
        if (iVar4 != 0) goto LAB_00ea3118;
        if (-1 < (int)puVar3) goto LAB_00ea30c6;
      }
      bVar5 = -1 < (int)puVar2;
      puVar3 = puVar2;
      puVar2 = puVar1;
      if (bVar5) goto LAB_00ea30c6;
    }
    puVar3 = puVar2;
    if (-1 < (int)puVar2) {
LAB_00ea30c6:
      FUN_00ea01e0(param_1,puVar3);
      ExceptionList = local_c;
      return param_1;
    }
    ppuVar6 = &PTR_017f8e10;
  }
  FUN_004398f0(param_1,(wchar_t *)ppuVar6);
  ExceptionList = local_c;
  return param_1;
}



