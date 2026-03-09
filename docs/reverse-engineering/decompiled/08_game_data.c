/*
 * SGW.exe Decompilation - 08_game_data
 * Classes: 12
 * Stargate Worlds Client
 */


/* ========== CookedCharCreationData.c ========== */

/*
 * SGW.exe - CookedCharCreationData (2 functions)
 */

/* Also in vtable: CookedCharCreationData (slot 0) */

undefined4 * __thiscall CookedCharCreationData__vfunc_0(void *this,byte param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167a943;
  pvStack_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &pvStack_c;
  FUN_00450d70((void *)((int)this + 0xc));
  local_4 = 0xffffffff;
  FUN_00424130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvStack_c;
  return this;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* [VTable] CookedCharCreationData virtual function #1
   VTable: vtable_CookedCharCreationData at 017fbad8 */

uint __thiscall CookedCharCreationData__vfunc_1(void *this,undefined4 param_1)

{
  int iVar1;
  void *this_00;
  undefined4 uVar2;
  uint uVar3;
  int *piVar4;
  int iVar5;
  void **ppvVar6;
  undefined **appuStack_17208 [2];
  int *piStack_17200;
  int *piStack_171fc;
  undefined4 uStack_171f8;
  undefined4 uStack_171f4;
  void *pvStack_171f0;
  int *piStack_171ec;
  short *psStack_171e8;
  code *pcStack_171e4;
  short asStack_171e0 [2];
  undefined4 uStack_171dc;
  undefined4 uStack_171d8;
  undefined4 uStack_14170;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016fd83b;
  local_c = ExceptionList;
  appuStack_17208[0] = CookedData::CookedData__CharCreateCharDefSetType::vftable;
  piStack_17200 = (int *)0x0;
  piStack_171fc = (int *)0x0;
  uStack_171f8 = 0;
  uStack_171f4 = 0;
  local_4 = 1;
  ExceptionList = &local_c;
  FUN_004241c0(asStack_171e0,0x1e21998);
  local_4._0_1_ = 2;
  psStack_171e8 = asStack_171e0;
  pcStack_171e4 = FUN_00a459a0;
  uStack_171d8 = 0x8000;
  uStack_171dc = 0x8000;
  FUN_00a3bdb0((int)asStack_171e0);
  uStack_14170 = param_1;
  FUN_00a48220((char *)asStack_171e0);
  local_4._0_1_ = 3;
  iVar1 = FUN_015e78e0(appuStack_17208,(char *)asStack_171e0,(byte *)"COOKED_CHAR_CREATION",0);
  if (iVar1 == 0) {
    local_4._0_1_ = 2;
    FUN_00a459a0((char *)asStack_171e0);
    FUN_00a3b980((int)asStack_171e0);
    local_4 = CONCAT31(local_4._1_3_,1);
    FUN_00424220(asStack_171e0);
    local_4 = 0xffffffff;
    uVar3 = FUN_00d35840(appuStack_17208);
    uVar3 = uVar3 & 0xffffff00;
  }
  else {
    iVar1 = FUN_00c66ad0();
    iVar1 = __RTDynamicCast(*(undefined4 *)(*(int *)(iVar1 + 0x8c) + 4),0,
                            &GameEntityBase::RTTI_Type_Descriptor,&GameAccount::RTTI_Type_Descriptor
                            ,0);
    piVar4 = piStack_17200;
    if (iVar1 != 0) {
      if (piStack_171fc < piStack_17200) {
        _invalid_parameter_noinfo();
      }
      piStack_171ec = piStack_171fc;
      if (piStack_171fc < piStack_17200) {
        _invalid_parameter_noinfo();
      }
      for (; piVar4 != piStack_171ec; piVar4 = piVar4 + 1) {
        if (piStack_171fc <= piVar4) {
          _invalid_parameter_noinfo();
        }
        iVar5 = *piVar4;
        if (this != (void *)0x0) {
          *(int *)((int)this + 4) = *(int *)((int)this + 4) + 1;
        }
        local_4._0_1_ = 4;
        ppvVar6 = &pvStack_171f0;
        pvStack_171f0 = this;
        this_00 = (void *)FUN_00e73590(iVar1);
        FUN_00d332d0(this_00,iVar5,(int *)ppvVar6);
        local_4._0_1_ = 3;
        if ((this != (void *)0x0) &&
           (*(int *)((int)this + 4) = *(int *)((int)this + 4) + -1, *(int *)((int)this + 4) < 1)) {
          (*(code *)**(undefined4 **)this)(1);
        }
        if (piStack_171fc <= piVar4) {
          _invalid_parameter_noinfo();
        }
      }
    }
    local_4._0_1_ = 2;
    FUN_00a459a0((char *)asStack_171e0);
    FUN_00a3b980((int)asStack_171e0);
    local_4 = CONCAT31(local_4._1_3_,1);
    FUN_00424220(asStack_171e0);
    local_4 = 0xffffffff;
    uVar2 = FUN_00d35840(appuStack_17208);
    uVar3 = CONCAT31((int3)((uint)uVar2 >> 8),1);
  }
  ExceptionList = local_c;
  return uVar3;
}




/* ========== CookedData_CookedData.c ========== */

/*
 * SGW.exe - CookedData_CookedData (26 functions)
 */

/* Also in vtable: SGWTableOfContents__toc__ModuleType_LayoutInstance (slot 0) */

undefined4 CookedData_CookedData__ItemRangeSetType__vfunc_0(void)

{
  return 0x18;
}


/* Also in vtable: SGWTableOfContents_action__BindingSaveType (slot 0) */

undefined4 CookedData_CookedData__DialogScreenType__vfunc_0(void)

{
  return 0x12;
}


undefined4 CookedData_CookedData__ObjectTextType__vfunc_0(void)

{
  return 0x1f;
}


/* Also in vtable: UTextureCube (slot 72) */

undefined4 CookedData_CookedData__ErrorTextType__vfunc_0(void)

{
  return 0x20;
}


undefined4 CookedData_CookedData__SpecialWords__vfunc_0(void)

{
  return 0x2b;
}


/* Also in vtable: SGWTableOfContents__toc__ModuleType_Metadata (slot 0) */

undefined4 CookedData_CookedData__ItemType__vfunc_0(void)

{
  return 0x1a;
}


undefined4 CookedData_CookedData__ContainerType__vfunc_0(void)

{
  return 0x23;
}


/* Also in vtable: SGWTableOfContents__action__actionGroups (slot 0) */

undefined4 CookedData_CookedData__DialogType__vfunc_0(void)

{
  return 0x13;
}


undefined4 CookedData_CookedData__StargateType__vfunc_0(void)

{
  return 0x22;
}


undefined4 CookedData_CookedData__WorldInfoType__vfunc_0(void)

{
  return 0x21;
}


/* Also in vtable: SGWTableOfContents___cmd__CommandList_sequence (slot 0) */

undefined4 CookedData_CookedData__AppliedScienceType__vfunc_0(void)

{
  return 0x27;
}


/* Also in vtable: SGWTableOfContents__action__ActionGroupType_action (slot 0) */

undefined4 CookedData_CookedData__RacialParadigmType__vfunc_0(void)

{
  return 0x29;
}


undefined4 CookedData_CookedData__BlueprintType__vfunc_0(void)

{
  return 0x26;
}


/* Also in vtable: 00___TStaticMeshVertexData (slot 2) */

undefined4 CookedData_CookedData__DisciplineType__vfunc_0(void)

{
  return 0x28;
}


undefined4 CookedData_CookedData__InteractionType__vfunc_0(void)

{
  return 0x2d;
}


undefined4 CookedData_CookedData__BehaviorEventType__vfunc_0(void)

{
  return 0x2c;
}


undefined4 CookedData_CookedData__DisciplineListType__vfunc_0(void)

{
  return 0x19;
}


undefined4 CookedData_CookedData__ItemRequirementsSetType__vfunc_0(void)

{
  return 0x16;
}


undefined4 CookedData_CookedData__ItemEventSetType__vfunc_0(void)

{
  return 0x17;
}


undefined4 CookedData_CookedData__BlueprintComponentType__vfunc_0(void)

{
  return 0x24;
}


/* Also in vtable: SGWTableOfContents__action__ActionGroupType_action_defaultBind (slot 0) */

undefined4 CookedData_CookedData__SpecialWordType__vfunc_0(void)

{
  return 0x2a;
}


undefined4 CookedData_CookedData__CharCreateVisualGroupType__vfunc_0(void)

{
  return 0x1c;
}


undefined4 CookedData_CookedData__CharCreateCharDefType__vfunc_0(void)

{
  return 0x1d;
}


undefined4 CookedData_CookedData__BlueprintComponentListType__vfunc_0(void)

{
  return 0x25;
}


undefined4 CookedData_CookedData__CharCreateChoiceType__vfunc_0(void)

{
  return 0x1b;
}


undefined4 CookedData_CookedData__CharCreateCharDefSetType__vfunc_0(void)

{
  return 0x1e;
}




/* ========== CookedElementBase.c ========== */

/*
 * SGW.exe - CookedElementBase (1 functions)
 */

/* Also in vtable: CookedElementBase (slot 0) */

undefined4 * __thiscall CookedElementBase__vfunc_0(void *this,byte param_1)

{
  FUN_00424130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CookedErrorTextType.c ========== */

/*
 * SGW.exe - CookedErrorTextType (2 functions)
 */

/* Also in vtable: CookedErrorTextType (slot 0) */

undefined4 * __thiscall CookedErrorTextType__vfunc_0(void *this,byte param_1)

{
  FUN_004344f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* [VTable] CookedErrorTextType virtual function #1
   VTable: vtable_CookedErrorTextType at 017fb6b4 */

undefined1 __thiscall CookedErrorTextType__vfunc_1(void *this,undefined4 param_1)

{
  wchar_t *pwVar1;
  int iVar2;
  uint uVar3;
  undefined1 uStack_1720d;
  undefined **ppuStack_1720c;
  undefined4 uStack_17208;
  undefined4 uStack_17204;
  wchar_t *pwStack_17200;
  undefined4 uStack_171fc;
  undefined4 uStack_171f8;
  undefined4 uStack_171f4;
  wchar_t *pwStack_171f0;
  undefined4 uStack_171ec;
  short *psStack_171e8;
  code *pcStack_171e4;
  short asStack_171e0 [2];
  undefined4 uStack_171dc;
  undefined4 uStack_171d8;
  undefined4 uStack_14170;
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  puStack_8 = &LAB_016fa1b1;
  pvStack_c = ExceptionList;
  uStack_1720d = 0;
  ppuStack_1720c = CookedData::CookedData__ErrorTextType::vftable;
  uStack_17208 = 0;
  uStack_17204 = 0;
  pwStack_17200 = (wchar_t *)0x0;
  uStack_171fc = 0;
  uStack_171f8 = 0;
  uStack_171f4 = 0;
  pwStack_171f0 = (wchar_t *)0x0;
  uStack_171ec = 0;
  local_4 = 0;
  ExceptionList = &pvStack_c;
  FUN_004241c0(asStack_171e0,0x1e21998);
  local_4._0_1_ = 1;
  psStack_171e8 = asStack_171e0;
  pcStack_171e4 = FUN_00a459a0;
  uStack_171d8 = 0x8000;
  uStack_171dc = 0x8000;
  FUN_00a3bdb0((int)asStack_171e0);
  uStack_14170 = param_1;
  FUN_00a48220((char *)asStack_171e0);
  local_4 = CONCAT31(local_4._1_3_,2);
  iVar2 = FUN_015e0ac0(&ppuStack_1720c,(char *)asStack_171e0,(byte *)"COOKED_ERROR_TEXT",0);
  pwVar1 = pwStack_17200;
  if (iVar2 != 0) {
    *(undefined4 *)((int)this + 0x34) = uStack_171f8;
    *(undefined4 *)((int)this + 0x10) = uStack_17204;
    *(undefined4 *)((int)this + 0xc) = uStack_17208;
    *(undefined4 *)((int)this + 0x38) = uStack_171f4;
    *(undefined4 *)((int)this + 0x30) = uStack_171fc;
    if (pwStack_17200 != (wchar_t *)0x0) {
      uVar3 = std::char_traits<wchar_t>::length(pwStack_17200);
      FUN_00438520((void *)((int)this + 0x14),pwVar1,uVar3);
    }
    pwVar1 = pwStack_171f0;
    if (pwStack_171f0 != (wchar_t *)0x0) {
      uVar3 = std::char_traits<wchar_t>::length(pwStack_171f0);
      FUN_00438520((void *)((int)this + 0x3c),pwVar1,uVar3);
    }
    uStack_1720d = 1;
  }
  local_4._0_1_ = 1;
  FUN_00a459a0((char *)asStack_171e0);
  FUN_00a3b980((int)asStack_171e0);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_00424220(asStack_171e0);
  ExceptionList = pvStack_c;
  return uStack_1720d;
}




/* ========== CookedKismetEventSequenceData.c ========== */

/*
 * SGW.exe - CookedKismetEventSequenceData (2 functions)
 */

/* Also in vtable: CookedKismetEventSequenceData (slot 0) */

undefined4 * __thiscall CookedKismetEventSequenceData__vfunc_0(void *this,byte param_1)

{
  FUN_0043d920(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* [VTable] CookedKismetEventSequenceData virtual function #1
   VTable: vtable_CookedKismetEventSequenceData at 017fb910 */

bool __thiscall CookedKismetEventSequenceData__vfunc_1(void *this,undefined4 param_1)

{
  int iVar1;
  undefined **appuStack_1720c [2];
  undefined4 uStack_17204;
  undefined4 uStack_17200;
  undefined4 uStack_171fc;
  undefined4 uStack_171f8;
  undefined4 uStack_171f4;
  undefined4 uStack_171f0;
  undefined4 uStack_171ec;
  short *psStack_171e8;
  code *pcStack_171e4;
  short asStack_171e0 [2];
  undefined4 uStack_171dc;
  undefined4 uStack_171d8;
  undefined4 uStack_14170;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016fa729;
  local_c = ExceptionList;
  appuStack_1720c[0] = CookedData::CookedData__KismetEventSequenceType::vftable;
  uStack_17204 = 0;
  uStack_17200 = 0;
  uStack_171fc = 0;
  uStack_171f8 = 0;
  uStack_171f4 = 0;
  uStack_171f0 = 0;
  uStack_171ec = 0;
  local_4 = 1;
  ExceptionList = &local_c;
  FUN_004241c0(asStack_171e0,0x1e21998);
  local_4._0_1_ = 2;
  psStack_171e8 = asStack_171e0;
  pcStack_171e4 = FUN_00a459a0;
  uStack_171d8 = 0x8000;
  uStack_171dc = 0x8000;
  FUN_00a3bdb0((int)asStack_171e0);
  uStack_14170 = param_1;
  FUN_00a48220((char *)asStack_171e0);
  local_4 = CONCAT31(local_4._1_3_,3);
  iVar1 = FUN_015e7a40(appuStack_1720c,(char *)asStack_171e0,(byte *)"COOKED_KISMET_EVENT_SEQUENCE",
                       0);
  if (iVar1 != 0) {
    FUN_00d04d60(this,(int)appuStack_1720c);
  }
  local_4._0_1_ = 2;
  FUN_00a459a0((char *)asStack_171e0);
  FUN_00a3b980((int)asStack_171e0);
  local_4 = CONCAT31(local_4._1_3_,1);
  FUN_00424220(asStack_171e0);
  local_4 = 0xffffffff;
  FUN_00d0a550(appuStack_1720c);
  ExceptionList = local_c;
  return iVar1 != 0;
}




/* ========== CookedKismetEventSetData.c ========== */

/*
 * SGW.exe - CookedKismetEventSetData (2 functions)
 */

/* Also in vtable: CookedKismetEventSetData (slot 0) */

undefined4 * __thiscall CookedKismetEventSetData__vfunc_0(void *this,byte param_1)

{
  FUN_0043b4c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* [VTable] CookedKismetEventSetData virtual function #1
   VTable: vtable_CookedKismetEventSetData at 017fb8f0 */

undefined1 __thiscall CookedKismetEventSetData__vfunc_1(void *this,undefined4 param_1)

{
  int *piVar1;
  int iVar2;
  undefined4 *puVar3;
  undefined1 uVar4;
  int *piVar5;
  char *pcStack_1721c;
  undefined **appuStack_17218 [2];
  int *piStack_17210;
  int *piStack_1720c;
  undefined4 uStack_17208;
  undefined4 uStack_17204;
  undefined4 uStack_17200;
  undefined4 *puStack_171fc;
  short *psStack_171f8;
  code *pcStack_171f4;
  undefined4 *puStack_171f0;
  undefined **appuStack_171ec [3];
  short asStack_171e0 [2];
  undefined4 uStack_171dc;
  undefined4 uStack_171d8;
  undefined4 uStack_14170;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016fa79a;
  local_c = ExceptionList;
  uVar4 = 0;
  appuStack_17218[0] = CookedData::CookedData__KismetEventSetType::vftable;
  piStack_17210 = (int *)0x0;
  piStack_1720c = (int *)0x0;
  uStack_17208 = 0;
  uStack_17204 = 0;
  uStack_17200 = 0;
  local_4 = 1;
  ExceptionList = &local_c;
  pcStack_1721c = this;
  FUN_004241c0(asStack_171e0,0x1e21998);
  local_4._0_1_ = 2;
  psStack_171f8 = asStack_171e0;
  pcStack_171f4 = FUN_00a459a0;
  uStack_171d8 = 0x8000;
  uStack_171dc = 0x8000;
  FUN_00a3bdb0((int)asStack_171e0);
  uStack_14170 = param_1;
  FUN_00a48220((char *)asStack_171e0);
  local_4 = CONCAT31(local_4._1_3_,3);
  iVar2 = FUN_015e7a20(appuStack_17218,(char *)asStack_171e0,(byte *)"COOKED_KISMET_EVENT_SET",0);
  piVar5 = piStack_17210;
  if (iVar2 != 0) {
    *(undefined4 *)((int)this + 0x1c) = uStack_17204;
    if (piStack_1720c < piStack_17210) {
      _invalid_parameter_noinfo();
    }
    while( true ) {
      piVar1 = piStack_1720c;
      if (piStack_1720c < piStack_17210) {
        _invalid_parameter_noinfo();
      }
      if (piVar5 == piVar1) break;
      puVar3 = (undefined4 *)scalable_malloc(0x40);
      if (puVar3 == (undefined4 *)0x0) {
        pcStack_1721c = "scalable_malloc failed";
        std::exception::exception((exception *)appuStack_171ec,&pcStack_1721c);
        appuStack_171ec[0] = std::bad_alloc::vftable;
        local_4 = CONCAT31((int3)((uint)local_4 >> 8),3);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(appuStack_171ec,(ThrowInfo *)&DAT_01d65cc8);
      }
      local_4._0_1_ = 5;
      puStack_171f0 = puVar3;
      puVar3 = FUN_0043d890(puVar3);
      if (puVar3 != (undefined4 *)0x0) {
        puVar3[1] = puVar3[1] + 1;
      }
      local_4 = CONCAT31(local_4._1_3_,6);
      puStack_171fc = puVar3;
      FUN_00d12140(pcStack_1721c + 0xc,(int *)&puStack_171fc);
      if (piStack_1720c <= piVar5) {
        _invalid_parameter_noinfo();
      }
      FUN_00d04d60(puVar3,*piVar5);
      local_4 = CONCAT31(local_4._1_3_,3);
      if ((puVar3 != (undefined4 *)0x0) && (puVar3[1] = puVar3[1] + -1, (int)puVar3[1] < 1)) {
        (**(code **)*puVar3)(1);
      }
      if (piStack_1720c <= piVar5) {
        _invalid_parameter_noinfo();
      }
      piVar5 = piVar5 + 1;
    }
    uVar4 = 1;
  }
  local_4._0_1_ = 2;
  FUN_00a459a0((char *)asStack_171e0);
  FUN_00a3b980((int)asStack_171e0);
  local_4._0_1_ = 7;
  FUN_00a41260(asStack_171e0);
  FUN_00a49110(asStack_171e0);
  FUN_00a3b510(asStack_171e0,0);
  local_4 = CONCAT31(local_4._1_3_,1);
  FUN_00a49ef0(asStack_171e0);
  appuStack_17218[0] = CookedData::CookedData__KismetEventSetType::vftable;
  local_4 = 0xffffffff;
  if (piStack_17210 == (int *)0x0) {
    ExceptionList = local_c;
    return uVar4;
  }
                    /* WARNING: Subroutine does not return */
  scalable_free(piStack_17210);
}




/* ========== CookedSpecialWordsType.c ========== */

/*
 * SGW.exe - CookedSpecialWordsType (2 functions)
 */

/* Also in vtable: CookedSpecialWordsType (slot 0) */

undefined4 * __thiscall CookedSpecialWordsType__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01679da3;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_0043a940((int)this + 0xc);
  local_4 = 0xffffffff;
  FUN_00424130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* [VTable] CookedSpecialWordsType virtual function #1
   VTable: vtable_CookedSpecialWordsType at 017fb900 */

undefined1 CookedSpecialWordsType__vfunc_1(undefined4 param_1)

{
  wchar_t *pwVar1;
  int *piVar2;
  int iVar3;
  uint uVar4;
  undefined1 uVar5;
  int *piVar6;
  undefined **appuStack_17234 [2];
  int *piStack_1722c;
  int *piStack_17228;
  undefined4 uStack_17224;
  undefined4 uStack_17220;
  int iStack_1721c;
  wchar_t awStack_17218 [2];
  short *psStack_17214;
  code *pcStack_17210;
  undefined4 uStack_1720c;
  undefined4 uStack_17208;
  undefined4 uStack_17204;
  undefined4 uStack_17200;
  undefined1 auStack_171fc [4];
  wchar_t awStack_171f8 [8];
  undefined4 uStack_171e8;
  undefined4 uStack_171e4;
  short asStack_171e0 [2];
  undefined4 uStack_171dc;
  undefined4 uStack_171d8;
  undefined4 uStack_14170;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016fa23e;
  pvStack_c = ExceptionList;
  uVar5 = 0;
  appuStack_17234[0] = CookedData::CookedData__SpecialWords::vftable;
  piStack_1722c = (int *)0x0;
  piStack_17228 = (int *)0x0;
  uStack_17224 = 0;
  uStack_17220 = 0;
  local_4 = 1;
  ExceptionList = &pvStack_c;
  FUN_004241c0(asStack_171e0,0x1e21998);
  local_4._0_1_ = 2;
  psStack_17214 = asStack_171e0;
  pcStack_17210 = FUN_00a459a0;
  uStack_171d8 = 0x8000;
  uStack_171dc = 0x8000;
  FUN_00a3bdb0((int)asStack_171e0);
  uStack_14170 = param_1;
  FUN_00a48220((char *)asStack_171e0);
  local_4 = CONCAT31(local_4._1_3_,3);
  iVar3 = FUN_015e7860(appuStack_17234,(char *)asStack_171e0,(byte *)"COOKED_SPECIAL_WORDS",0);
  piVar6 = piStack_1722c;
  if (iVar3 != 0) {
    if (piStack_17228 < piStack_1722c) {
      _invalid_parameter_noinfo();
    }
    while( true ) {
      piVar2 = piStack_17228;
      if (piStack_17228 < piStack_1722c) {
        _invalid_parameter_noinfo();
      }
      if (&stack0x00000000 == (undefined1 *)0x17230) {
        _invalid_parameter_noinfo();
      }
      if (piVar6 == piVar2) break;
      if (&stack0x00000000 == (undefined1 *)0x17230) {
        _invalid_parameter_noinfo();
      }
      if (piStack_17228 <= piVar6) {
        _invalid_parameter_noinfo();
      }
      iVar3 = *piVar6;
      uStack_171e4 = 7;
      awStack_17218[0] = L'\0';
      awStack_17218[1] = L'\0';
      uStack_171e8 = 0;
      std::char_traits<wchar_t>::assign(awStack_171f8,awStack_17218);
      local_4._0_1_ = 5;
      uStack_1720c = *(undefined4 *)(iVar3 + 4);
      uStack_17208 = *(undefined4 *)(iVar3 + 8);
      uStack_17204 = *(undefined4 *)(iVar3 + 0xc);
      uStack_17200 = *(undefined4 *)(iVar3 + 0x10);
      pwVar1 = *(wchar_t **)(iVar3 + 0x14);
      if (pwVar1 != (wchar_t *)0x0) {
        uVar4 = std::char_traits<wchar_t>::length(pwVar1);
        FUN_00438520(auStack_171fc,pwVar1,uVar4);
      }
      FUN_00d00350((void *)(iStack_1721c + 0xc),&uStack_1720c);
      local_4 = CONCAT31(local_4._1_3_,3);
      FUN_00434320((int)&uStack_1720c);
      if (piStack_17228 <= piVar6) {
        _invalid_parameter_noinfo();
      }
      piVar6 = piVar6 + 1;
    }
    uVar5 = 1;
  }
  local_4._0_1_ = 2;
  FUN_00a459a0((char *)asStack_171e0);
  FUN_00a3b980((int)asStack_171e0);
  local_4._0_1_ = 6;
  FUN_00a41260(asStack_171e0);
  FUN_00a49110(asStack_171e0);
  FUN_00a3b510(asStack_171e0,0);
  local_4 = CONCAT31(local_4._1_3_,1);
  FUN_00a49ef0(asStack_171e0);
  appuStack_17234[0] = CookedData::CookedData__SpecialWords::vftable;
  local_4 = 0xffffffff;
  if (piStack_1722c != (int *)0x0) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  ExceptionList = pvStack_c;
  return uVar5;
}




/* ========== CookedTextType.c ========== */

/*
 * SGW.exe - CookedTextType (2 functions)
 */

/* Also in vtable: CookedTextType (slot 0) */

undefined4 * __thiscall CookedTextType__vfunc_0(void *this,byte param_1)

{
  FUN_004345b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* [VTable] CookedTextType virtual function #1
   VTable: vtable_CookedTextType at 017fb6c4 */

undefined1 __thiscall CookedTextType__vfunc_1(void *this,undefined4 param_1)

{
  wchar_t *pwVar1;
  int iVar2;
  uint uVar3;
  undefined1 uStack_17209;
  undefined **ppuStack_17208;
  undefined4 uStack_17204;
  wchar_t *pwStack_17200;
  undefined4 uStack_171fc;
  undefined4 uStack_171f8;
  undefined4 uStack_171f4;
  wchar_t *pwStack_171f0;
  undefined4 uStack_171ec;
  short *psStack_171e8;
  code *pcStack_171e4;
  short asStack_171e0 [2];
  undefined4 uStack_171dc;
  undefined4 uStack_171d8;
  undefined4 uStack_14170;
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  puStack_8 = &LAB_016fa1dc;
  pvStack_c = ExceptionList;
  uStack_17209 = 0;
  ppuStack_17208 = CookedData::CookedData__ObjectTextType::vftable;
  uStack_17204 = 0;
  pwStack_17200 = (wchar_t *)0x0;
  uStack_171fc = 0;
  uStack_171f8 = 0;
  uStack_171f4 = 0;
  pwStack_171f0 = (wchar_t *)0x0;
  uStack_171ec = 0;
  local_4 = 0;
  ExceptionList = &pvStack_c;
  FUN_004241c0(asStack_171e0,0x1e21998);
  local_4._0_1_ = 1;
  psStack_171e8 = asStack_171e0;
  pcStack_171e4 = FUN_00a459a0;
  uStack_171d8 = 0x8000;
  uStack_171dc = 0x8000;
  FUN_00a3bdb0((int)asStack_171e0);
  uStack_14170 = param_1;
  FUN_00a48220((char *)asStack_171e0);
  local_4 = CONCAT31(local_4._1_3_,2);
  iVar2 = FUN_015e0ae0(&ppuStack_17208,(char *)asStack_171e0,(byte *)"COOKED_TEXT",0);
  pwVar1 = pwStack_17200;
  if (iVar2 != 0) {
    *(undefined4 *)((int)this + 0x34) = uStack_171f4;
    *(undefined4 *)((int)this + 0x30) = uStack_171f8;
    *(undefined4 *)((int)this + 0x2c) = uStack_171fc;
    *(undefined4 *)((int)this + 0xc) = uStack_17204;
    if (pwStack_17200 != (wchar_t *)0x0) {
      uVar3 = std::char_traits<wchar_t>::length(pwStack_17200);
      FUN_00438520((void *)((int)this + 0x10),pwVar1,uVar3);
    }
    pwVar1 = pwStack_171f0;
    if (pwStack_171f0 != (wchar_t *)0x0) {
      uVar3 = std::char_traits<wchar_t>::length(pwStack_171f0);
      FUN_00438520((void *)((int)this + 0x38),pwVar1,uVar3);
    }
    uStack_17209 = 1;
  }
  local_4._0_1_ = 1;
  FUN_00a459a0((char *)asStack_171e0);
  FUN_00a3b980((int)asStack_171e0);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_00424220(asStack_171e0);
  ExceptionList = pvStack_c;
  return uStack_17209;
}




/* ========== DBEffect.c ========== */

/*
 * SGW.exe - DBEffect (1 functions)
 */

undefined4 * __thiscall DBEffect__vfunc_0(void *this,byte param_1)

{
  FUN_00d2d180(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DBGateInfo.c ========== */

/*
 * SGW.exe - DBGateInfo (1 functions)
 */

undefined4 * __thiscall DBGateInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d2d9c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DBInvContainer.c ========== */

/*
 * SGW.exe - DBInvContainer (1 functions)
 */

undefined4 * __thiscall DBInvContainer__vfunc_0(void *this,byte param_1)

{
  FUN_00d22460(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DBInvItem.c ========== */

/*
 * SGW.exe - DBInvItem (1 functions)
 */

undefined4 * __thiscall DBInvItem__vfunc_0(void *this,byte param_1)

{
  FUN_00d21980(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}



