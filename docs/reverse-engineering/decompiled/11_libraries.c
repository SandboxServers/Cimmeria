/*
 * SGW.exe Decompilation - 11_libraries
 * Classes: 30
 * Stargate Worlds Client
 */


/* ========== BMItem.c ========== */

/*
 * SGW.exe - BMItem (1 functions)
 */

undefined4 * __thiscall BMItem__vfunc_0(void *this,byte param_1)

{
  FUN_00e58910(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CFloat.c ========== */

/*
 * SGW.exe - CFloat (1 functions)
 */

undefined4 __thiscall CFloat__vfunc_0(void *this,undefined4 param_1,char *param_2)

{
  *(uint *)((int)this + 8) = (*param_2 != 'r') + 1;
  return 0;
}




/* ========== Concurrency.c ========== */

/*
 * SGW.exe - Concurrency (15 functions)
 */


/* Library Function - Single Match
    public: __thiscall Concurrency::cancellation_token_source::~cancellation_token_source(void)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

void __thiscall
Concurrency::cancellation_token_source::~cancellation_token_source(cancellation_token_source *this)

{
  if (*(int *)this != 0) {
    GWaitable::HandlerArray::Release(*(HandlerArray **)this);
  }
  return;
}




/* Library Function - Single Match
    public: virtual struct Concurrency::IScheduler * __thiscall
   Concurrency::details::UMSSchedulingContext::GetScheduler(void)
   
   Library: Visual Studio 2012 Debug
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 2) */

IScheduler * __thiscall
Concurrency::details::UMSSchedulingContext::GetScheduler(UMSSchedulingContext *this)

{
  IScheduler *pIVar1;
  
  pIVar1 = (IScheduler *)(**(code **)(*(int *)(*(int *)(this + 4) + 0x14) + 8))();
  return pIVar1;
}




/* Library Function - Single Match
    public: virtual struct Concurrency::IScheduler * __thiscall
   Concurrency::details::UMSSchedulingContext::GetScheduler(void)
   
   Library: Visual Studio 2012 Debug
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 3) */

IScheduler * __thiscall
Concurrency::details::UMSSchedulingContext::GetScheduler(UMSSchedulingContext *this)

{
  IScheduler *pIVar1;
  
  pIVar1 = (IScheduler *)(**(code **)(*(int *)(*(int *)(this + 4) + 0x14) + 0xc))();
  return pIVar1;
}




/* Library Function - Single Match
    public: virtual struct Concurrency::IScheduler * __thiscall
   Concurrency::details::UMSSchedulingContext::GetScheduler(void)
   
   Library: Visual Studio 2012 Debug
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 4) */

IScheduler * __thiscall
Concurrency::details::UMSSchedulingContext::GetScheduler(UMSSchedulingContext *this)

{
  IScheduler *pIVar1;
  
  pIVar1 = (IScheduler *)(**(code **)(*(int *)(*(int *)(this + 4) + 0x14) + 0x10))();
  return pIVar1;
}




/* Library Function - Single Match
    public: void __thiscall Concurrency::details::CollectionTypes::Count::Increment(void)
   
   Library: Visual Studio */

void __thiscall Concurrency::details::CollectionTypes::Count::Increment(Count *this)

{
  *(int *)this = *(int *)this + 1;
  return;
}




/* Library Function - Single Match
    public: __thiscall Concurrency::cancellation_token_source::~cancellation_token_source(void)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

void __thiscall
Concurrency::cancellation_token_source::~cancellation_token_source(cancellation_token_source *this)

{
  if (*(int *)this != 0) {
    FUN_013c9c70(*(undefined4 **)this);
  }
  return;
}




/* Library Function - Single Match
    public: void __thiscall Concurrency::cancellation_token_source::cancel(void)const 
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

void __thiscall Concurrency::cancellation_token_source::cancel(cancellation_token_source *this)

{
  FUN_013e0060(*(int **)this);
  return;
}




/* Library Function - Single Match
    public: __thiscall Concurrency::cancellation_token_source::~cancellation_token_source(void)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

void __thiscall
Concurrency::cancellation_token_source::~cancellation_token_source(cancellation_token_source *this)

{
  if (*(int *)this != 0) {
    FUN_00455900(*(int **)this);
  }
  return;
}




/* Library Function - Single Match
    public: __thiscall Concurrency::details::QuickBitSet::QuickBitSet(void)
   
   Libraries: Visual Studio 2012, Visual Studio 2015, Visual Studio 2017, Visual Studio 2019 */

QuickBitSet * __thiscall Concurrency::details::QuickBitSet::QuickBitSet(QuickBitSet *this)

{
  *(undefined4 *)this = 0;
  *(undefined4 *)(this + 4) = 0;
  return this;
}




/* Library Function - Single Match
    private: bool __thiscall Concurrency::details::LockQueueNode::IsTicketValid(void)
   
   Library: Visual Studio 2010 Debug */

bool __thiscall Concurrency::details::LockQueueNode::IsTicketValid(LockQueueNode *this)

{
  return (*(uint *)(this + 8) & 2) != 0;
}




/* Library Function - Single Match
    private: bool __thiscall Concurrency::details::LockQueueNode::IsBlocked(void)
   
   Library: Visual Studio 2010 Debug */

bool __thiscall Concurrency::details::LockQueueNode::IsBlocked(LockQueueNode *this)

{
  return (*(uint *)(this + 8) & 1) != 0;
}




/* Library Function - Single Match
    public: __thiscall Concurrency::details::SchedulerBase::NumaInformation::NumaInformation(void)
   
   Libraries: Visual Studio 2012, Visual Studio 2015, Visual Studio 2017, Visual Studio 2019 */

NumaInformation * __thiscall
Concurrency::details::SchedulerBase::NumaInformation::NumaInformation(NumaInformation *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_01781678;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  QuickBitSet::QuickBitSet((QuickBitSet *)this);
  local_8 = 0;
  FUN_014074b0((IUnknown *)(this + 0xc));
  ExceptionList = local_10;
  return this;
}




/* Library Function - Single Match
    public: __thiscall Concurrency::details::WorkItem::WorkItem(void)
   
   Library: Visual Studio */

WorkItem * __thiscall Concurrency::details::WorkItem::WorkItem(WorkItem *this)

{
  *(undefined4 *)this = 0;
  *(undefined4 *)(this + 8) = 0;
  return this;
}




/* Library Function - Single Match
    public: virtual __thiscall
   Concurrency::details::_AsyncTaskCollection::~_AsyncTaskCollection(void)
   
   Library: Visual Studio 2012 Debug */

void __thiscall
Concurrency::details::_AsyncTaskCollection::~_AsyncTaskCollection(_AsyncTaskCollection *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  puStack_c = &LAB_017886c8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  local_8 = 0;
  FUN_013f9180((int *)(this + 8));
  local_8 = 0xffffffff;
  FUN_0146ff50((cancellation_token_source *)this);
  ExceptionList = local_10;
  return;
}




/* Library Function - Single Match
    public: void __thiscall Concurrency::details::_NonReentrantLock::_Release(void)
   
   Library: Visual Studio */

void __thiscall Concurrency::details::_NonReentrantLock::_Release(_NonReentrantLock *this)

{
  *(uint *)this = *(uint *)this & 0xfffffffe;
  return;
}





/* ========== DelphiInterface.c ========== */

/*
 * SGW.exe - DelphiInterface (13 functions)
 */

/* [VTable] DelphiInterface virtual function #11
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_11(void *this,undefined4 param_1)

{
  (**(code **)((int)this + 0x7c))(param_1);
  return;
}


/* [VTable] DelphiInterface virtual function #3
   VTable: vtable_DelphiInterface at 01921dbc */

void __fastcall DelphiInterface__vfunc_3(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x00a32933. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(param_1 + 0x38))();
  return;
}


/* [VTable] DelphiInterface virtual function #4
   VTable: vtable_DelphiInterface at 01921dbc */

void DelphiInterface__vfunc_4(void)

{
  return;
}


/* [VTable] DelphiInterface virtual function #8
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_8(void *this,undefined4 param_1)

{
  (**(code **)((int)this + 0x84))(param_1);
  return;
}


/* [VTable] DelphiInterface virtual function #9
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_9(void *this,undefined4 param_1)

{
  (**(code **)((int)this + 0x88))(param_1);
  return;
}


/* [VTable] DelphiInterface virtual function #5
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_5(void *this,LPCWSTR param_1)

{
  UINT *pUVar1;
  undefined1 local_94 [4];
  undefined1 auStack_90 [128];
  undefined1 *puStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d58eb;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  pUVar1 = FUN_00423f40(local_94,param_1);
  local_4 = 0;
  (**(code **)((int)this + 0x68))(pUVar1[0x21]);
  local_4 = 0xffffffff;
  if ((puStack_10 != auStack_90) && (puStack_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(puStack_10);
  }
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] DelphiInterface virtual function #1
   VTable: vtable_DelphiInterface at 01921dbc */

void __fastcall DelphiInterface__vfunc_1(int param_1)

{
  int *piVar1;
  BOOL BVar2;
  
  if (*(HMODULE *)(param_1 + 0x20) != (HMODULE)0x0) {
    BVar2 = FreeLibrary(*(HMODULE *)(param_1 + 0x20));
    if (BVar2 == 0) {
      return;
    }
    piVar1 = (int *)(param_1 + 0x90);
    *piVar1 = *piVar1 + -1;
    if (*piVar1 == 0) {
      FUN_00a32990(param_1);
    }
  }
  *(undefined4 *)(param_1 + 0x20) = 0;
  return;
}


/* [VTable] DelphiInterface virtual function #7
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_7(void *this,int *param_1)

{
  undefined **ppuVar1;
  UINT *pUVar2;
  int iVar3;
  int iVar4;
  undefined1 auStack_94 [4];
  undefined1 auStack_90 [128];
  undefined1 *puStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d5900;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  (**(code **)((int)this + 0x70))();
  iVar3 = 0;
  if (0 < param_1[1]) {
    iVar4 = 0;
    do {
      if (((undefined4 *)(*param_1 + iVar4))[1] == 0) {
        ppuVar1 = &PTR_017f8e10;
      }
      else {
        ppuVar1 = *(undefined ***)(*param_1 + iVar4);
      }
      pUVar2 = FUN_00423f40(auStack_94,(LPCWSTR)ppuVar1);
      uStack_4 = 0;
      (**(code **)((int)this + 0x74))(pUVar2[0x21]);
      uStack_4 = 0xffffffff;
      if ((puStack_10 != auStack_90) && (puStack_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
        scalable_free(puStack_10);
      }
      iVar3 = iVar3 + 1;
      iVar4 = iVar4 + 0xc;
    } while (iVar3 < param_1[1]);
  }
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] DelphiInterface virtual function #12
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_12(void *this,wchar_t *param_1,int param_2)

{
  int *piVar1;
  UINT *pUVar2;
  undefined **local_ac;
  int local_a8;
  int local_a0 [3];
  undefined1 local_94 [4];
  undefined1 auStack_90 [128];
  undefined1 *puStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d59bf;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_0041aab0(&local_ac,param_1);
  local_4 = 0;
  piVar1 = FUN_0048eeb0(&local_ac,local_a0);
  local_4._0_1_ = 1;
  FUN_0041a630(&local_ac,piVar1);
  local_4._0_1_ = 0;
  FUN_0041b420(local_a0);
  FUN_009f46a0(*(void **)(*(int *)((int)this + 0x10) + 0x80),param_1,param_2);
  if (local_a8 == 0) {
    local_ac = &PTR_017f8e10;
  }
  pUVar2 = FUN_00423f40(local_94,(LPCWSTR)local_ac);
  local_4._0_1_ = 2;
  (**(code **)((int)this + 0x5c))(pUVar2[0x21],param_2);
  local_4 = (uint)local_4._1_3_ << 8;
  if ((puStack_10 != auStack_90) && (puStack_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(puStack_10);
  }
  local_4 = 0xffffffff;
  FUN_0041b420((int *)&local_ac);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] DelphiInterface virtual function #13
   VTable: vtable_DelphiInterface at 01921dbc */

void __thiscall DelphiInterface__vfunc_13(void *this,wchar_t *param_1,int param_2)

{
  int *piVar1;
  UINT *pUVar2;
  undefined **local_ac;
  int local_a8;
  int local_a0 [3];
  undefined1 local_94 [4];
  undefined1 auStack_90 [128];
  undefined1 *puStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d59ea;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_0041aab0(&local_ac,param_1);
  local_4 = 0;
  piVar1 = FUN_0048eeb0(&local_ac,local_a0);
  local_4._0_1_ = 1;
  FUN_0041a630(&local_ac,piVar1);
  local_4._0_1_ = 0;
  FUN_0041b420(local_a0);
  FUN_009f68e0(*(void **)(*(int *)((int)this + 0x10) + 0x80),param_1,param_2);
  if (local_a8 == 0) {
    local_ac = &PTR_017f8e10;
  }
  pUVar2 = FUN_00423f40(local_94,(LPCWSTR)local_ac);
  local_4._0_1_ = 2;
  (**(code **)((int)this + 0x60))(pUVar2[0x21],param_2);
  local_4 = (uint)local_4._1_3_ << 8;
  if ((puStack_10 != auStack_90) && (puStack_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(puStack_10);
  }
  local_4 = 0xffffffff;
  FUN_0041b420((int *)&local_ac);
  ExceptionList = pvStack_c;
  return;
}


/* Also in vtable: DelphiInterface (slot 0) */

undefined4 __thiscall DelphiInterface__vfunc_0(void *this,int param_1)

{
  *(int *)((int)this + 0x10) = param_1;
  if (*(int *)((int)this + 0x90) == 0) {
    FUN_00a32c70((int)this);
    (**(code **)((int)this + 0x58))(&LAB_00a336e0);
    (**(code **)((int)this + 0x50))(*(undefined4 *)((int)this + 8));
    (**(code **)((int)this + 0x50))(*(undefined4 *)((int)this + 4));
    (**(code **)((int)this + 0x50))(*(undefined4 *)((int)this + 0xc));
  }
  (**(code **)(*(int *)this + 0xc))();
  return 1;
}


/* [VTable] DelphiInterface virtual function #14
   VTable: vtable_DelphiInterface at 01921dbc */

void __fastcall DelphiInterface__vfunc_14(void *param_1)

{
  int *this;
  int iVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int iVar4;
  int iVar5;
  int iStack_10;
  int iStack_c;
  
  this = (int *)((int)param_1 + 0x24);
  FUN_01102280(this);
  FUN_0041b3a0(&iStack_10);
  iVar5 = DAT_01edc69c;
  if (DAT_01edc6a0 <= iStack_c) {
LAB_00a33c04:
    (**(code **)((int)param_1 + 0x48))();
    (**(code **)((int)param_1 + 0x44))("Core..Object");
    if (DAT_01edc680 == (undefined4 *)0x0) {
      DAT_01edc680 = FUN_004a0450();
      FUN_0049fb30();
    }
    FUN_00a33860(param_1,DAT_01edc680);
    (**(code **)((int)param_1 + 0x4c))();
    return;
  }
  do {
    iVar4 = iStack_c;
    iVar1 = *(int *)(*(int *)(iVar5 + iStack_c * 4) + 0x3c);
    if (iVar1 != 0) {
      if (DAT_01edc708 == (undefined4 *)0x0) {
        DAT_01edc708 = FUN_004b47f0();
        FUN_004b4c80();
        iVar5 = DAT_01edc69c;
      }
      for (puVar2 = *(undefined4 **)(iVar1 + 0x34); puVar2 != (undefined4 *)0x0;
          puVar2 = (undefined4 *)puVar2[0xf]) {
        if (puVar2 == DAT_01edc708) goto LAB_00a33bb6;
      }
      if (DAT_01edc708 == (undefined4 *)0x0) {
LAB_00a33bb6:
        uVar3 = *(undefined4 *)(iVar5 + iVar4 * 4);
        if (*(int *)((int)param_1 + 0x30) == 0) {
          FUN_0104f9b0((int)this);
        }
        FUN_00503260(this,iVar1,uVar3);
        iVar5 = DAT_01edc69c;
      }
    }
    do {
      FUN_00419770(&iStack_10);
      if (DAT_01edc6a0 <= iStack_c) goto LAB_00a33c04;
    } while ((*(uint *)(*(int *)(iVar5 + iStack_c * 4) + 8) & 0x200) != 0);
  } while( true );
}


/* [VTable] DelphiInterface virtual function #2
   VTable: vtable_DelphiInterface at 01921dbc */

bool __fastcall DelphiInterface__vfunc_2(int param_1)

{
  return 0 < *(int *)(param_1 + 0x90);
}




/* ========== FCodec.c ========== */

/*
 * SGW.exe - FCodec (1 functions)
 */

/* [VTable] FCodec virtual function #2
   VTable: vtable_FCodec at 018e7b88 */

undefined4 * __thiscall FCodec__vfunc_2(void *this,byte param_1)

{
  *(undefined ***)this = FCodec::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FCodecBWT.c ========== */

/*
 * SGW.exe - FCodecBWT (2 functions)
 */

/* Also in vtable: FCodecBWT (slot 0) */

undefined4 FCodecBWT__vfunc_0(void)

{
  int iVar1;
  void *pvVar2;
  int iVar3;
  int iVar4;
  int iStack_4c;
  int iStack_48;
  void *pvStack_44;
  undefined4 uStack_40;
  int iStack_3c;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016bd510;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (DAT_01ea5778 == (int *)0x0) {
    iStack_3c = 0x8c585d;
    ExceptionList = &pvStack_c;
    FUN_00416660();
  }
  iStack_3c = 8;
  uStack_40 = 0x40000;
  pvStack_44 = (void *)0x0;
  iStack_48 = 0x8c586e;
  pvVar2 = (void *)(**(code **)(*DAT_01ea5778 + 8))();
  if (DAT_01ea5778 == (int *)0x0) {
    iStack_48 = 0x8c589a;
    FUN_00416660();
  }
  iStack_48 = 8;
  iStack_4c = 0x100004;
  iStack_3c = (**(code **)(*DAT_01ea5778 + 8))(0);
  DAT_01ee64c4 = 0;
  pvStack_44 = (void *)0x0;
  uStack_40 = 0;
  iVar3 = (**(code **)(iRam00040000 + 0x38))();
  while (iVar3 == 0) {
    iVar3 = (**(code **)(iRam00040000 + 0x34))();
    iVar4 = (**(code **)(iRam00040000 + 0x30))();
    DAT_01ee64c0 = iVar3 - iVar4;
    if (0x40000 < DAT_01ee64c0) {
      DAT_01ee64c0 = 0x40000;
    }
    (**(code **)(iRam00040000 + 4))(DAT_01ee64c4,DAT_01ee64c0);
    iVar3 = 0;
    if (DAT_01ee64c0 != -1 && -1 < DAT_01ee64c0 + 1) {
      do {
        *(int *)((int)pvStack_44 + iVar3 * 4) = iVar3;
        iVar3 = iVar3 + 1;
      } while (iVar3 < DAT_01ee64c0 + 1);
    }
    qsort(pvStack_44,DAT_01ee64c0 + 1,4,(_PtFuncCompare *)&LAB_008c54d0);
    iVar3 = 0;
    if (0 < DAT_01ee64c0 + 1) {
      do {
        iVar4 = *(int *)((int)pvStack_44 + iVar3 * 4);
        iVar1 = iVar3;
        if ((iVar4 != 1) && (iVar1 = iStack_4c, iVar4 == 0)) {
          iStack_48 = iVar3;
        }
        iStack_4c = iVar1;
        iVar3 = iVar3 + 1;
      } while (iVar3 < DAT_01ee64c0 + 1);
    }
    if (iRam0000003d == 0) {
      (**(code **)(iRam00000001 + 4))(&DAT_01ee64c0,4);
    }
    else {
      iVar3 = 3;
      do {
        (**(code **)(iRam00000001 + 4))((int)&DAT_01ee64c0 + iVar3,1);
        iVar3 = iVar3 + -1;
      } while (-1 < iVar3);
    }
    if (iRam0000003d == 0) {
      (**(code **)(iRam00000001 + 4))(&iStack_4c,4);
    }
    else {
      iVar3 = 3;
      do {
        (**(code **)(iRam00000001 + 4))((int)&iStack_4c + iVar3,1);
        iVar3 = iVar3 + -1;
      } while (-1 < iVar3);
    }
    if (iRam0000003d == 0) {
      (**(code **)(iRam00000001 + 4))(&iStack_48,4);
    }
    else {
      iVar3 = 3;
      do {
        (**(code **)(iRam00000001 + 4))((int)&iStack_48 + iVar3,1);
        iVar3 = iVar3 + -1;
      } while (-1 < iVar3);
    }
    iVar3 = 0;
    if (DAT_01ee64c0 != -1 && -1 < DAT_01ee64c0 + 1) {
      do {
        iVar4 = *(int *)((int)pvStack_44 + iVar3 * 4);
        if (iVar4 == 0) {
          iVar4 = 0;
        }
        else {
          iVar4 = iVar4 + -1;
        }
        (**(code **)(iRam00000001 + 4))(iVar4 + DAT_01ee64c4,1);
        iVar3 = iVar3 + 1;
      } while (iVar3 < DAT_01ee64c0 + 1);
    }
    iVar3 = (**(code **)(iRam00040000 + 0x38))();
  }
  FUN_004b7a90(&iStack_3c);
  FUN_0048ead0((int *)&stack0xffffffd0);
  ExceptionList = pvVar2;
  return 0;
}


/* [VTable] FCodecBWT virtual function #1
   VTable: vtable_FCodecBWT at 018e7bec */

undefined4 FCodecBWT__vfunc_1(int *param_1,int *param_2)

{
  int iVar1;
  int iVar2;
  uint uVar3;
  int unaff_ESI;
  int unaff_EDI;
  int *piVar4;
  int iStack_864;
  undefined4 uStack_860;
  int iStack_85c;
  int iStack_84c;
  int aiStack_848 [258];
  int aiStack_440 [261];
  void *pvStack_2c;
  undefined4 uStack_24;
  undefined4 uStack_18;
  void *pvStack_14;
  undefined1 *puStack_10;
  undefined4 uStack_c;
  
  uStack_c = 0xffffffff;
  puStack_10 = &LAB_016bd54c;
  pvStack_14 = ExceptionList;
  aiStack_848[1] = 0;
  aiStack_848[2] = 0x40001;
  aiStack_848[3] = 0x40001;
  ExceptionList = &pvStack_14;
  if (DAT_01ea5778 == (int *)0x0) {
    iStack_85c = 0x8c5b06;
    ExceptionList = &pvStack_14;
    FUN_00416660();
  }
  iStack_85c = 8;
  uStack_860 = 0x40001;
  iStack_864 = 0;
  (**(code **)(*DAT_01ea5778 + 8))();
  uStack_18 = 1;
  aiStack_848[2] = 0;
  aiStack_848[3] = 0x40001;
  aiStack_848[4] = 0x40001;
  if (DAT_01ea5778 == (int *)0x0) {
    FUN_00416660();
  }
  iStack_84c = (**(code **)(*DAT_01ea5778 + 8))(0);
  uStack_24 = CONCAT31(uStack_24._1_3_,3);
  iVar1 = (**(code **)(*param_1 + 0x38))();
  while (iVar1 == 0) {
    if (param_1[0xf] == 0) {
      (**(code **)(*param_1 + 4))(&iStack_864,4);
    }
    else {
      iVar1 = 3;
      do {
        (**(code **)(*param_1 + 4))((int)&iStack_864 + iVar1,1);
        iVar1 = iVar1 + -1;
      } while (-1 < iVar1);
    }
    if (param_1[0xf] == 0) {
      (**(code **)(*param_1 + 4))(&stack0xfffff7b0,4);
    }
    else {
      iVar1 = 3;
      do {
        (**(code **)(*param_1 + 4))((int)aiStack_848 + iVar1 + -8,1);
        iVar1 = iVar1 + -1;
      } while (-1 < iVar1);
    }
    if (param_1[0xf] == 0) {
      (**(code **)(*param_1 + 4))(&uStack_860,4);
    }
    else {
      iVar1 = 3;
      do {
        (**(code **)(*param_1 + 4))((int)&uStack_860 + iVar1,1);
        iVar1 = iVar1 + -1;
      } while (-1 < iVar1);
    }
    if (0x40001 < iStack_864) {
      FUN_00486000("DecompressLength<=MAX_BUFFER_SIZE+1",
                   "c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",0x50);
    }
    iVar1 = (**(code **)(*param_1 + 0x34))();
    iVar2 = (**(code **)(*param_1 + 0x30))();
    if (iVar1 - iVar2 < iStack_864) {
      FUN_00486000("DecompressLength<=In.TotalSize()-In.Tell()",
                   "c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",0x51);
    }
    iStack_864 = iStack_864 + 1;
    (**(code **)(*param_1 + 4))(iStack_85c,iStack_864);
    iVar1 = 0;
    piVar4 = aiStack_848;
    for (iVar2 = 0x101; iVar2 != 0; iVar2 = iVar2 + -1) {
      *piVar4 = 0;
      piVar4 = piVar4 + 1;
    }
    do {
      if (iVar1 == 8) {
        uVar3 = 0x100;
      }
      else {
        uVar3 = (uint)*(byte *)(iStack_864 + iVar1);
      }
      aiStack_848[uVar3] = aiStack_848[uVar3] + 1;
      iVar1 = iVar1 + 1;
    } while (iVar1 < 0x100004);
    iVar2 = 0;
    iVar1 = 0;
    do {
      *(int *)((int)aiStack_440 + iVar1) = iVar2;
      iVar2 = iVar2 + *(int *)((int)aiStack_848 + iVar1);
      *(undefined4 *)((int)aiStack_848 + iVar1) = 0;
      iVar1 = iVar1 + 4;
    } while (iVar1 < 0x404);
    iVar1 = 0;
    do {
      if (iVar1 == 8) {
        uVar3 = 0x100;
      }
      else {
        uVar3 = (uint)*(byte *)(iStack_864 + iVar1);
      }
      iVar2 = aiStack_848[uVar3];
      aiStack_848[uVar3] = iVar2 + 1;
      *(int *)(unaff_ESI + (aiStack_440[uVar3] + iVar2) * 4) = iVar1;
      iVar1 = iVar1 + 1;
    } while (iVar1 < 0x100004);
    iVar2 = 0;
    iVar1 = unaff_EDI;
    do {
      (**(code **)(*param_2 + 4))(iStack_864 + iVar1,1);
      iVar1 = *(int *)(unaff_ESI + iVar1 * 4);
      iVar2 = iVar2 + 1;
    } while (iVar2 < 0x100003);
    iVar1 = (**(code **)(*param_1 + 0x38))();
  }
  uStack_24._1_3_ = (undefined3)((uint)uStack_24 >> 8);
  uStack_24 = CONCAT31(uStack_24._1_3_,1);
  FUN_004b7a90(&iStack_84c);
  uStack_24 = 0xffffffff;
  FUN_0048ead0(&iStack_85c);
  ExceptionList = pvStack_2c;
  return 1;
}




/* ========== FCodecFull.c ========== */

/*
 * SGW.exe - FCodecFull (3 functions)
 */

/* [VTable] FCodecFull virtual function #2
   VTable: vtable_FCodecFull at 018e7c74 */

undefined4 * __thiscall FCodecFull__vfunc_2(void *this,byte param_1)

{
  FUN_008c5ec0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: FCodecFull (slot 0) */

undefined4 __thiscall FCodecFull__vfunc_0(void *this,undefined ***param_1,undefined ***param_2)

{
  FUN_008c6640(this,param_1,param_2,1,0,&LAB_008c5f40);
  return 0;
}


/* [VTable] FCodecFull virtual function #1
   VTable: vtable_FCodecFull at 018e7c74 */

undefined4 __thiscall FCodecFull__vfunc_1(void *this,undefined ***param_1,undefined ***param_2)

{
  FUN_008c6640(this,param_1,param_2,0xffffffff,*(int *)((int)this + 8) + -1,&LAB_008c51d0);
  return 1;
}




/* ========== FCodecHuffman.c ========== */

/*
 * SGW.exe - FCodecHuffman (2 functions)
 */

/* [VTable] FCodecHuffman virtual function #1
   VTable: vtable_FCodecHuffman at 018e7c14 */

undefined4 FCodecHuffman__vfunc_1(int *param_1)

{
  int *piVar1;
  int iVar2;
  int iVar3;
  uint uVar4;
  int unaff_EBX;
  int *unaff_retaddr;
  void *pvStack_c0;
  int local_bc;
  undefined4 uStack_b8;
  int aiStack_b4 [8];
  undefined **appuStack_94 [27];
  int aiStack_28 [5];
  void *pvStack_14;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016bd6a4;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_0047f0e0(param_1,(int)&local_bc,4);
  iVar2 = (**(code **)(*param_1 + 0x34))();
  iVar3 = (**(code **)(*param_1 + 0x30))();
  FUN_00757880(&uStack_b8,iVar2 - iVar3);
  uStack_4 = 0;
  (**(code **)(*param_1 + 4))(uStack_b8,aiStack_b4[0]);
  FUN_0156c260(appuStack_94,pvStack_c0,local_bc * 8);
  pvStack_c._0_1_ = 1;
  FUN_008c5dc0(aiStack_b4,0xffffffff);
  pvStack_c = (void *)CONCAT31(pvStack_c._1_3_,2);
  FUN_008c6240(aiStack_b4,(char *)appuStack_94);
  while (0 < unaff_EBX) {
    unaff_EBX = unaff_EBX + -1;
    iVar2 = FBitReader__vfunc_14((int)appuStack_94);
    if (iVar2 != 0) {
      FUN_00486000("!Reader.AtEnd()",
                   "c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",0x127);
    }
    piVar1 = aiStack_b4;
    iVar2 = aiStack_b4[0];
    while (iVar2 == -1) {
      uVar4 = UActorChannel__unknown_0156baa0((int *)appuStack_94);
      piVar1 = *(int **)(piVar1[2] + (uVar4 & 0xff) * 4);
      iVar2 = *piVar1;
    }
    (**(code **)(*unaff_retaddr + 4))(&stack0xffffff3b,1);
  }
  pvStack_c._1_3_ = (undefined3)((uint)pvStack_c >> 8);
  pvStack_c._0_1_ = 1;
  FCodecHuffman__unknown_008c5e30((int)aiStack_b4);
  pvStack_c = (void *)CONCAT31(pvStack_c._1_3_,3);
  FUN_0048ead0(aiStack_28);
  appuStack_94[0] = FArchive::vftable;
  pvStack_c = (void *)0xffffffff;
  FUN_0048ead0((int *)&pvStack_c0);
  ExceptionList = pvStack_14;
  return 1;
}


/* WARNING: Removing unreachable block (ram,0x008c6e33) */
/* WARNING: Removing unreachable block (ram,0x008c6ca5) */
/* WARNING: Removing unreachable block (ram,0x008c6f7c) */
/* Also in vtable: FCodecHuffman (slot 0) */

void FCodecHuffman__vfunc_0(int *param_1)

{
  int *piVar1;
  undefined4 uVar2;
  undefined1 *puVar3;
  undefined *puVar4;
  int iVar5;
  int *piVar6;
  int iVar7;
  int iVar8;
  int unaff_ESI;
  int iVar9;
  int unaff_EDI;
  undefined4 uStack_e0;
  int *piStack_cc;
  int *piStack_c8;
  int *piStack_c4;
  int iStack_c0;
  char *pcStack_bc;
  int *piStack_b8;
  undefined **ppuStack_b4;
  undefined **ppuStack_b0;
  undefined4 uStack_ac;
  undefined1 auStack_a8 [4];
  undefined1 auStack_a4 [4];
  int aiStack_a0 [11];
  int iStack_74;
  undefined4 uStack_34;
  int *piStack_28;
  undefined1 uStack_18;
  undefined1 uStack_14;
  undefined4 uStack_10;
  int *piStack_c;
  int *piStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  piStack_8 = (int *)&LAB_016bd9c7;
  piStack_c = ExceptionList;
  uStack_e0 = 0x8c6a40;
  ExceptionList = &piStack_c;
  ppuStack_b0 = (undefined **)(**(code **)(*param_1 + 0x30))();
  uStack_ac = 0;
  piStack_c8 = (int *)0x0;
  piStack_c4 = (int *)0x100;
  iStack_c0 = 0x100;
  if (DAT_01ea5778 == (int *)0x0) {
    uStack_e0 = 0x8c6a6a;
    FUN_00416660();
  }
  uStack_e0 = 8;
  iVar5 = (**(code **)(*DAT_01ea5778 + 8))();
  uStack_10._1_3_ = 0;
  iVar9 = 0;
  do {
    uStack_10._0_1_ = 1;
    piVar6 = (int *)scalable_malloc(0x20);
    if (piVar6 == (int *)0x0) {
      pcStack_bc = "scalable_malloc failed";
      std::exception::exception((exception *)&ppuStack_b0,&pcStack_bc);
      ppuStack_b0 = std::bad_alloc::vftable;
      uStack_10 = CONCAT31(uStack_10._1_3_,1);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(&ppuStack_b0,(ThrowInfo *)&DAT_01d65cc8);
    }
    *piVar6 = iVar9;
    piVar6[1] = 0;
    piVar6[2] = 0;
    piVar6[3] = 0;
    piVar6[4] = 0;
    piStack_c4 = piVar6 + 5;
    *piStack_c4 = 0;
    piVar6[6] = 0;
    piVar6[7] = 0;
    uStack_10._0_1_ = 1;
    *(int **)(iVar5 + iVar9 * 4) = piVar6;
    iVar9 = iVar9 + 1;
    piStack_c8 = piVar6;
  } while (iVar9 < 0x100);
  FUN_01051c60(auStack_a4,(int *)&stack0xffffff2c);
  uStack_10 = CONCAT31(uStack_10._1_3_,8);
  iVar9 = (**(code **)(*param_1 + 0x38))();
  while (iVar9 == 0) {
    (**(code **)(*param_1 + 4))(&stack0xffffff2b,1);
    piVar6 = (int *)(*(int *)(unaff_EDI + (uStack_e0 >> 0x18) * 4) + 4);
    *piVar6 = *piVar6 + 1;
    iStack_c0 = iStack_c0 + 1;
    iVar9 = (**(code **)(*param_1 + 0x38))();
  }
  (**(code **)(*param_1 + 0x3c))(pcStack_bc);
  piVar6 = piStack_8;
  if (piStack_8[0xf] == 0) {
    (**(code **)(*piStack_8 + 4))(&pcStack_bc,4);
  }
  else {
    iVar9 = 3;
    do {
      (**(code **)(*piVar6 + 4))((int)&pcStack_bc + iVar9,1);
      iVar9 = iVar9 + -1;
    } while (-1 < iVar9);
  }
  while (1 < iVar5) {
    if (unaff_ESI == 0) {
      FUN_00486000("Data","c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                   0x302);
    }
    if (iVar5 < 1) {
      FUN_00486000("c<ArrayNum",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x303);
    }
    if (*(int *)(*(int *)(unaff_ESI + -4 + iVar5 * 4) + 4) != 0) break;
    if (iVar5 < 1) {
      FUN_00486000("ArrayNum>0",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x2ee);
    }
    iVar7 = *(int *)(unaff_ESI + -4 + iVar5 * 4);
    iVar9 = iVar5 + -1;
    if (iVar9 < 0) {
      FUN_00486000("Index>=0",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x3ad);
    }
    if (iVar5 < iVar9) {
      FUN_00486000("Index<=ArrayNum",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x3ae);
    }
    FUN_00483580(&stack0xffffff28,iVar9,1,4,8);
    if (iVar7 != 0) {
      FCodecHuffman__unknown_008c5e30(iVar7);
                    /* WARNING: Subroutine does not return */
      scalable_free(iVar7);
    }
  }
  iVar9 = iVar5 * 9;
  do {
    piStack_c4 = (int *)iVar9;
    if (iVar5 < 2) {
      if (iVar5 < 1) {
        FUN_00486000("ArrayNum>0",
                     "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x2ee)
        ;
      }
      puVar3 = *(undefined1 **)(unaff_ESI + -4 + iVar5 * 4);
      iVar7 = iVar5 + -1;
      if (iVar7 < 0) {
        FUN_00486000("Index>=0",
                     "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x3ad)
        ;
      }
      if (iVar5 < iVar7) {
        FUN_00486000("Index<=ArrayNum",
                     "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",0x3ae)
        ;
      }
      FUN_00483580(&stack0xffffff28,iVar7,1,4,8);
      iVar5 = (**(code **)(*param_1 + 0x38))();
      piVar6 = piStack_c4;
      while (iVar5 == 0) {
        (**(code **)(*param_1 + 4))(&stack0xffffff27,1);
        iVar9 = iVar9 + *(int *)(*ppuStack_b0 + 0x18);
        iVar5 = (**(code **)(*param_1 + 0x38))();
        piVar6 = (int *)iVar9;
      }
      piStack_c4 = piVar6;
      (**(code **)(*param_1 + 0x3c))(iStack_c0);
      FUN_0156c150(aiStack_a0,iVar9);
      uStack_18 = 0xf;
      UNetConnection__unknown_0156bfb0(aiStack_a0,*(int *)(puVar3 + 0xc) != 0);
      if (*(int *)(puVar3 + 0xc) == 0) {
        uStack_e0 = CONCAT13(*puVar3,(undefined3)uStack_e0);
        (**(code **)(aiStack_a0[0] + 4))((int)&uStack_e0 + 3,1);
      }
      else {
        iVar5 = 0;
        if (0 < *(int *)(puVar3 + 0xc)) {
          do {
            FUN_008c5770(*(void **)(*(int *)(puVar3 + 8) + iVar5 * 4),aiStack_a0);
            iVar5 = iVar5 + 1;
          } while (iVar5 < *(int *)(puVar3 + 0xc));
        }
      }
      iVar5 = (**(code **)(*param_1 + 0x38))();
      while (iVar5 == 0) {
        (**(code **)(*param_1 + 4))((int)&uStack_e0 + 3,1);
        puVar4 = *ppuStack_b4;
        iVar5 = 0;
        if (0 < *(int *)(puVar4 + 0x18)) {
          do {
            UNetConnection__unknown_0156bfb0(auStack_a8,*(char *)(*(int *)(puVar4 + 0x14) + iVar5));
            iVar5 = iVar5 + 1;
          } while (iVar5 < *(int *)(puVar4 + 0x18));
        }
        iVar5 = (**(code **)(*param_1 + 0x38))();
      }
      if (iStack_74 != 0) {
        FUN_00486000("!Writer.IsError()",
                     "c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",0x114);
      }
      if (piStack_28 != piStack_c8) {
        FUN_00486000("Writer.GetNumBits()==BitCount",
                     "c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",0x115);
      }
      (**(code **)(*piStack_c + 4))(uStack_34,(int)piStack_28 + 7 >> 3);
      FCodecHuffman__unknown_008c5e30((int)puVar3);
                    /* WARNING: Subroutine does not return */
      scalable_free(puVar3);
    }
    piVar6 = (int *)scalable_malloc(0x20);
    iVar9 = 0;
    piStack_c8 = piVar6;
    if (piVar6 == (int *)0x0) {
      piStack_cc = (int *)0x17f7e00;
      std::exception::exception((exception *)&ppuStack_b4,(char **)&piStack_cc);
      ppuStack_b4 = std::bad_alloc::vftable;
      uStack_14 = 8;
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(&ppuStack_b4,(ThrowInfo *)&DAT_01d65cc8);
    }
    piVar1 = piVar6 + 2;
    *piVar6 = -1;
    piVar6[1] = 0;
    *piVar1 = 0;
    piVar6[3] = 0;
    piVar6[4] = 0;
    piStack_b8 = piVar6 + 5;
    *piStack_b8 = 0;
    piVar6[6] = 0;
    piVar6[7] = 0;
    uStack_14 = 8;
    piVar6[3] = piVar6[3] + 2;
    iVar7 = piVar6[3];
    piStack_cc = piVar6;
    if (piVar6[4] < iVar7) {
      iVar8 = *piVar1;
      iVar7 = ((int)(iVar7 * 3 + (iVar7 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar7;
      piVar6[4] = iVar7;
      if ((iVar8 != 0) || (iVar7 != 0)) {
        piStack_cc = (int *)(iVar7 * 4);
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        iVar7 = (**(code **)(*DAT_01ea5778 + 8))(iVar8,piStack_cc,8);
        *piVar1 = iVar7;
      }
    }
    if (0 < piVar6[3]) {
      do {
        if (iVar5 < 1) {
          FUN_00486000("ArrayNum>0",
                       "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                       0x2ee);
        }
        uVar2 = *(undefined4 *)(unaff_ESI + -4 + iVar5 * 4);
        iVar7 = iVar5 + -1;
        if (iVar7 < 0) {
          FUN_00486000("Index>=0",
                       "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                       0x3ad);
        }
        if (iVar5 < iVar7) {
          FUN_00486000("Index<=ArrayNum",
                       "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                       0x3ae);
        }
        FUN_00483580(&stack0xffffff28,iVar7,1,4,8);
        *(undefined4 *)(*piVar1 + iVar9 * 4) = uVar2;
        iVar7 = *(int *)(*piVar1 + iVar9 * 4);
        FUN_0041a820((undefined4 *)(iVar7 + 0x14),0,1,1,8);
        iVar8 = 0;
        **(undefined1 **)(iVar7 + 0x14) = (char)iVar9;
        if (0 < *(int *)(iVar7 + 0xc)) {
          do {
            FUN_008c6200(*(void **)(*(int *)(iVar7 + 8) + iVar8 * 4),iVar9);
            iVar8 = iVar8 + 1;
          } while (iVar8 < *(int *)(iVar7 + 0xc));
        }
        piStack_c8[1] = piStack_c8[1] + *(int *)(*(int *)(*piVar1 + iVar9 * 4) + 4);
        iVar9 = iVar9 + 1;
      } while (iVar9 < piStack_c8[3]);
    }
    iVar7 = 0;
    if (0 < iVar5) {
      do {
        if (*(int *)(*(int *)(unaff_ESI + iVar7 * 4) + 4) < piStack_c8[1]) break;
        iVar7 = iVar7 + 1;
      } while (iVar7 < iVar5);
    }
    FUN_0041a820(&stack0xffffff28,iVar7,1,4,8);
    iVar9 = (int)piStack_c4 + 1;
    *(int **)(unaff_ESI + iVar7 * 4) = piStack_c8;
    param_1 = piStack_c;
  } while( true );
}




/* ========== FCodecMTF.c ========== */

/*
 * SGW.exe - FCodecMTF (2 functions)
 */

/* Also in vtable: FCodecMTF (slot 0) */

undefined4 FCodecMTF__vfunc_0(int *param_1,int *param_2)

{
  int iVar1;
  uint unaff_EBX;
  undefined1 auStack_102 [2];
  undefined1 auStack_100 [256];
  
  iVar1 = 0;
  do {
    auStack_100[iVar1] = (char)iVar1;
    iVar1 = iVar1 + 1;
  } while (iVar1 < 0x100);
  iVar1 = (**(code **)(*param_1 + 0x38))();
  do {
    if (iVar1 != 0) {
      return 0;
    }
    (**(code **)(*param_1 + 4))(auStack_102,1);
    iVar1 = 0;
    do {
      if (auStack_100[iVar1 + -8] == (char)(unaff_EBX >> 0x10)) {
        if (iVar1 < 0x100) goto LAB_008c5608;
        break;
      }
      iVar1 = iVar1 + 1;
    } while (iVar1 < 0x100);
    FUN_00486000("i<256","c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",0x145)
    ;
LAB_008c5608:
    unaff_EBX = unaff_EBX & 0xffffff;
    (**(code **)(*param_2 + 4))(&stack0xfffffef7,1);
    for (; 0 < iVar1; iVar1 = iVar1 + -1) {
      auStack_100[iVar1 + -0x10] = auStack_100[iVar1 + -0x11];
    }
    iVar1 = (**(code **)(*param_1 + 0x38))();
  } while( true );
}


/* [VTable] FCodecMTF virtual function #1
   VTable: vtable_FCodecMTF at 018e7bfc */

undefined4 FCodecMTF__vfunc_1(int *param_1,int *param_2)

{
  int iVar1;
  uint uVar2;
  undefined4 uStack_114;
  undefined4 uStack_110;
  undefined1 uStack_101;
  undefined1 auStack_100 [256];
  
  iVar1 = 0;
  do {
    auStack_100[iVar1] = (char)iVar1;
    iVar1 = iVar1 + 1;
  } while (iVar1 < 0x100);
  iVar1 = (**(code **)(*param_1 + 0x38))();
  while (iVar1 == 0) {
    uStack_110 = 1;
    uStack_114 = &uStack_101;
    (**(code **)(*param_1 + 4))();
    (**(code **)(*param_2 + 4))(&stack0xfffffef6,1);
    uVar2 = (uint)uStack_114 >> 0x18;
    if (uVar2 != 0) {
      do {
        *(undefined1 *)((int)&uStack_110 + uVar2) = *(undefined1 *)((int)&uStack_114 + uVar2 + 3);
        uVar2 = uVar2 - 1;
      } while (0 < (int)uVar2);
    }
    uStack_110 = CONCAT31(uStack_110._1_3_,uStack_114._2_1_);
    iVar1 = (**(code **)(*param_1 + 0x38))();
  }
  return 1;
}




/* ========== FCodecRLE.c ========== */

/*
 * SGW.exe - FCodecRLE (2 functions)
 */

/* Also in vtable: FCodecRLE (slot 0) */

undefined4 FCodecRLE__vfunc_0(int *param_1,int *param_2)

{
  int iVar1;
  char unaff_BL;
  byte bVar2;
  undefined4 unaff_EBP;
  uint uVar3;
  char cVar4;
  undefined1 uStack_d;
  uint local_c;
  uint local_8;
  undefined4 local_4;
  
  bVar2 = 0;
  local_c = local_c & 0xffffff00;
  local_8 = local_8 & 0xffffff00;
  iVar1 = (**(code **)(*param_1 + 0x38))();
  do {
    if (iVar1 != 0) {
      FUN_008c5250(param_2,local_c,(byte)local_8);
      return 0;
    }
    (**(code **)(*param_1 + 4))(&uStack_d,1);
    cVar4 = (char)((uint)unaff_EBP >> 0x18);
    if ((cVar4 != unaff_BL) || (bVar2 == 0xff)) {
      uVar3 = (uint)bVar2;
      local_4 = CONCAT31(local_4._1_3_,bVar2);
      if (uVar3 < 6) {
        if (uVar3 != 0) goto LAB_008c5320;
      }
      else {
        uVar3 = 5;
LAB_008c5320:
        do {
          (**(code **)(*param_2 + 4))(&stack0x00000000,1);
          uVar3 = uVar3 - 1;
        } while (0 < (int)uVar3);
      }
      if (4 < (byte)local_4) {
        (**(code **)(*param_2 + 4))(&local_4,1);
      }
      bVar2 = 0;
      unaff_BL = cVar4;
    }
    bVar2 = bVar2 + 1;
    iVar1 = (**(code **)(*param_1 + 0x38))();
  } while( true );
}


/* [VTable] FCodecRLE virtual function #1
   VTable: vtable_FCodecRLE at 018e7b98 */

undefined4 FCodecRLE__vfunc_1(int *param_1,int *param_2)

{
  int iVar1;
  char cVar2;
  byte unaff_BP;
  undefined4 unaff_ESI;
  int iVar3;
  char cVar4;
  undefined1 uStack_1;
  
  iVar3 = 0;
  cVar2 = '\0';
  iVar1 = (**(code **)(*param_1 + 0x38))();
  while (iVar1 == 0) {
    (**(code **)(*param_1 + 4))(&uStack_1,1);
    (**(code **)(*param_2 + 4))(&stack0xfffffff7,1);
    cVar4 = (char)((uint)unaff_ESI >> 0x18);
    if (cVar4 == cVar2) {
      iVar3 = iVar3 + 1;
      if (iVar3 == 5) {
        (**(code **)(*param_1 + 4))(&stack0xfffffff4,1);
        if (unaff_BP < 2) {
          FUN_00486000("C>=2","c:\\BUILD\\QA\\SGW\\Working\\Development\\Src\\Core\\Inc\\FCodec.h",
                       0x9b);
        }
        for (; 5 < unaff_BP; unaff_BP = unaff_BP - 1) {
          (**(code **)(*param_2 + 4))(&stack0xffffffef,1);
        }
        unaff_BP = unaff_BP - 1;
        iVar3 = 0;
      }
    }
    else {
      iVar3 = 1;
      cVar2 = cVar4;
    }
    iVar1 = (**(code **)(*param_1 + 0x38))();
  }
  return 1;
}




/* ========== NVI_Image32F.c ========== */

/*
 * SGW.exe - NVI_Image32F (1 functions)
 */

undefined4 __thiscall NVI_Image32F__vfunc_0(void *this,uint param_1,uint param_2)

{
  int *this_00;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01771fdb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined4 *)((int)this + 4) = 0;
  *(uint *)((int)this + 8) = param_1;
  *(uint *)((int)this + 0xc) = param_2;
  this_00 = (int *)FUN_00418e30(0xc);
  local_4 = 0;
  if (this_00 == (int *)0x0) {
    *(undefined4 *)((int)this + 4) = 0;
  }
  else {
    *this_00 = 0;
    this_00[1] = 0;
    this_00[2] = 0;
    if ((param_2 != 0) || (param_1 != 0)) {
      FFileManagerError__unknown_0125f730(this_00);
      FFileManagerError__unknown_0125f780(this_00,param_2,param_1);
    }
    *(int **)((int)this + 4) = this_00;
  }
  ExceptionList = local_c;
  return 0;
}




/* ========== NormalMapMaker32F.c ========== */

/*
 * SGW.exe - NormalMapMaker32F (1 functions)
 */

undefined4 * __thiscall NormalMapMaker32F__vfunc_0(void *this,byte param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)((int)this + 4);
  *(undefined ***)this = NormalMapMaker32F::vftable;
  if (puVar1 != (undefined4 *)0x0) {
    FUN_0129bd30(puVar1);
                    /* WARNING: Subroutine does not return */
    scalable_free(puVar1);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Random.c ========== */

/*
 * SGW.exe - Random (1 functions)
 */

undefined4 * __thiscall Random__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Random::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ReferenceCount.c ========== */

/*
 * SGW.exe - ReferenceCount (1 functions)
 */

/* Also in vtable: ReferenceCount (slot 0) */

undefined4 * __thiscall ReferenceCount__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = ReferenceCount::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== SafeReferenceCount.c ========== */

/*
 * SGW.exe - SafeReferenceCount (1 functions)
 */

/* Also in vtable: SafeReferenceCount (slot 0) */

undefined4 * __thiscall SafeReferenceCount__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_0167b828;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = ReferenceCount::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== TiXmlAttribute.c ========== */

/*
 * SGW.exe - TiXmlAttribute (1 functions)
 */

undefined4 * __thiscall TiXmlAttribute__vfunc_0(void *this,byte param_1)

{
  FUN_01612b40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlBase.c ========== */

/*
 * SGW.exe - TiXmlBase (1 functions)
 */

undefined4 * __thiscall TiXmlBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = TiXmlBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlComment.c ========== */

/*
 * SGW.exe - TiXmlComment (1 functions)
 */

undefined4 * __thiscall TiXmlComment__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = TiXmlComment::vftable;
  FUN_01612d60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlDeclaration.c ========== */

/*
 * SGW.exe - TiXmlDeclaration (1 functions)
 */

undefined4 * __thiscall TiXmlDeclaration__vfunc_0(void *this,byte param_1)

{
  FUN_01613950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlDocument.c ========== */

/*
 * SGW.exe - TiXmlDocument (1 functions)
 */

CMFCToolBarsListCheckBox * __thiscall TiXmlDocument__vfunc_0(void *this,uint param_1)

{
  CMFCToolBarsListCheckBox::~CMFCToolBarsListCheckBox(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlElement.c ========== */

/*
 * SGW.exe - TiXmlElement (1 functions)
 */

undefined4 * __thiscall TiXmlElement__vfunc_0(void *this,byte param_1)

{
  FUN_01613b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlNode.c ========== */

/*
 * SGW.exe - TiXmlNode (1 functions)
 */

undefined4 * __thiscall TiXmlNode__vfunc_0(void *this,byte param_1)

{
  FUN_01612d60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlText.c ========== */

/*
 * SGW.exe - TiXmlText (1 functions)
 */

undefined4 * __thiscall TiXmlText__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = TiXmlText::vftable;
  FUN_01612d60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TiXmlUnknown.c ========== */

/*
 * SGW.exe - TiXmlUnknown (1 functions)
 */

undefined4 * __thiscall TiXmlUnknown__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = TiXmlUnknown::vftable;
  FUN_01612d60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VirtualMachine_ObjectRef.c ========== */

/*
 * SGW.exe - VirtualMachine_ObjectRef (1 functions)
 */

/* Also in vtable: VirtualMachine_ObjectRef (slot 0) */

undefined4 * __thiscall VirtualMachine_ObjectRef__vfunc_0(void *this,byte param_1)

{
  FUN_0093bd30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== WTInterface.c ========== */

/*
 * SGW.exe - WTInterface (14 functions)
 */

/* [VTable] WTInterface virtual function #11
   VTable: vtable_WTInterface at 019220f4 */

void WTInterface__vfunc_11(undefined4 param_1)

{
  (*DAT_01eeced8)(6,param_1,0,0,0);
  return;
}


/* [VTable] WTInterface virtual function #5
   VTable: vtable_WTInterface at 019220f4 */

void WTInterface__vfunc_5(undefined4 param_1)

{
  (*DAT_01eeced8)(0xc,0,0,param_1,0);
  return;
}


/* [VTable] WTInterface virtual function #3
   VTable: vtable_WTInterface at 019220f4 */

void __fastcall WTInterface__vfunc_3(int param_1)

{
  if (*(int *)(*(int *)(param_1 + 0x10) + 0x54) != 0) {
    (*DAT_01eeced8)(0,0,0,0,0);
  }
  return;
}


/* [VTable] WTInterface virtual function #4
   VTable: vtable_WTInterface at 019220f4 */

void WTInterface__vfunc_4(void)

{
  return;
}


/* [VTable] WTInterface virtual function #7
   VTable: vtable_WTInterface at 019220f4 */

void __fastcall WTInterface__vfunc_7(int *param_1)

{
  (**(code **)(*param_1 + 0x3c))();
  return;
}


/* [VTable] WTInterface virtual function #8
   VTable: vtable_WTInterface at 019220f4 */

void __thiscall WTInterface__vfunc_8(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x14) = 1;
  (*DAT_01eeced8)(0x13,param_1,0,0,0);
  return;
}


/* [VTable] WTInterface virtual function #9
   VTable: vtable_WTInterface at 019220f4 */

void __thiscall WTInterface__vfunc_9(void *this,undefined4 param_1)

{
  (*DAT_01eeced8)(0x14,param_1,0,0,0);
  *(undefined4 *)((int)this + 0x14) = 0;
  return;
}


/* [VTable] WTInterface virtual function #1
   VTable: vtable_WTInterface at 019220f4 */

void __fastcall WTInterface__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  (*DAT_01eeced8)(0x16,0,0,0,0);
  puVar1 = *(undefined4 **)(param_1 + 0x3c);
  if (puVar1 != (undefined4 *)0x0) {
    (*DAT_01eeced8)(0x16,0,0,0,0);
    FreeLibrary((HMODULE)*puVar1);
                    /* WARNING: Subroutine does not return */
    scalable_free(puVar1);
  }
  return;
}


/* [VTable] WTInterface virtual function #15
   VTable: vtable_WTInterface at 019220f4 */

void __fastcall WTInterface__vfunc_15(int *param_1)

{
  undefined4 uVar1;
  int *piVar2;
  undefined4 *puVar3;
  undefined **ppuVar4;
  int iVar5;
  int iVar6;
  int iVar7;
  int aiStack_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d5b00;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  (*DAT_01eeced8)(0xe,0,0,0,0);
  piVar2 = *(int **)(param_1[4] + 0x84);
  if (0 < *piVar2) {
    iVar6 = 0;
    iVar5 = 0;
    do {
      if ((((-1 < iVar5) && (iVar5 < piVar2[3])) && (iVar7 = piVar2[2] + iVar6, iVar7 != 0)) &&
         ((*(int *)(iVar7 + 4) != 0 && (*(int *)(*(int *)(iVar7 + 4) + 4) != 0)))) {
        if (*(int *)(iVar7 + 0xc) == 0) {
          FUN_00486000("Data",
                       "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                       0x309);
        }
        uVar1 = *(undefined4 *)(*(int *)(iVar7 + 0xc) + -4 + *(int *)(iVar7 + 0x10) * 4);
        puVar3 = FUN_004a0be0(*(void **)(*(int *)(iVar7 + 4) + 4),aiStack_18,(void *)0x0);
        uStack_4 = 0;
        if (puVar3[1] == 0) {
          ppuVar4 = &PTR_017f8e10;
        }
        else {
          ppuVar4 = (undefined **)*puVar3;
        }
        (*DAT_01eeced8)(0xf,uVar1,0,ppuVar4,0);
        uStack_4 = 0xffffffff;
        FUN_0041b420(aiStack_18);
      }
      piVar2 = *(int **)(param_1[4] + 0x84);
      iVar5 = iVar5 + 1;
      iVar6 = iVar6 + 0x40;
    } while (iVar5 < *piVar2);
  }
  (**(code **)(*param_1 + 0xc))();
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] WTInterface virtual function #12
   VTable: vtable_WTInterface at 019220f4 */

void __thiscall WTInterface__vfunc_12(void *this,wchar_t *param_1,int param_2)

{
  int *piVar1;
  undefined **local_24;
  int local_20;
  int local_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d5b1a;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_0041aab0(&local_24,param_1);
  local_4 = 0;
  piVar1 = FUN_0048eeb0(&local_24,local_18);
  local_4._0_1_ = 1;
  FUN_0041a630(&local_24,piVar1);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_0041b420(local_18);
  FUN_009f46a0(*(void **)(*(int *)((int)this + 0x10) + 0x80),param_1,param_2);
  if (local_20 == 0) {
    local_24 = &PTR_017f8e10;
  }
  (*DAT_01eeced8)(9,param_2,0,local_24,0);
  local_4 = 0xffffffff;
  FUN_0041b420((int *)&local_24);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] WTInterface virtual function #13
   VTable: vtable_WTInterface at 019220f4 */

void __thiscall WTInterface__vfunc_13(void *this,wchar_t *param_1,int param_2)

{
  int *piVar1;
  undefined **local_24;
  int local_20;
  int local_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d5b34;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_0041aab0(&local_24,param_1);
  local_4 = 0;
  piVar1 = FUN_0048eeb0(&local_24,local_18);
  local_4._0_1_ = 1;
  FUN_0041a630(&local_24,piVar1);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_0041b420(local_18);
  FUN_009f68e0(*(void **)(*(int *)((int)this + 0x10) + 0x80),param_1,param_2);
  if (local_20 == 0) {
    local_24 = &PTR_017f8e10;
  }
  (*DAT_01eeced8)(10,param_2,0,local_24,0);
  local_4 = 0xffffffff;
  FUN_0041b420((int *)&local_24);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] WTInterface virtual function #14
   VTable: vtable_WTInterface at 019220f4 */

void __fastcall WTInterface__vfunc_14(void *param_1)

{
  int *this;
  int iVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int iVar4;
  int iVar5;
  int iStack_10;
  int iStack_c;
  
  this = (int *)((int)param_1 + 0x28);
  FUN_01102280(this);
  FUN_0041b3a0(&iStack_10);
  iVar5 = DAT_01edc69c;
  if (DAT_01edc6a0 <= iStack_c) {
LAB_00a34e74:
    (*DAT_01eeced8)(4,0,0,0,0);
    (*DAT_01eeced8)(3,0,0,L"Core..Object",0);
    if (DAT_01edc680 == (undefined4 *)0x0) {
      DAT_01edc680 = FUN_004a0450();
      FUN_0049fb30();
    }
    FUN_00a34b40(param_1,DAT_01edc680);
    (*DAT_01eeced8)(5,0,0,0,0);
    return;
  }
  do {
    iVar4 = iStack_c;
    iVar1 = *(int *)(*(int *)(iVar5 + iStack_c * 4) + 0x3c);
    if (iVar1 != 0) {
      if (DAT_01edc708 == (undefined4 *)0x0) {
        DAT_01edc708 = FUN_004b47f0();
        FUN_004b4c80();
        iVar5 = DAT_01edc69c;
      }
      for (puVar2 = *(undefined4 **)(iVar1 + 0x34); puVar2 != (undefined4 *)0x0;
          puVar2 = (undefined4 *)puVar2[0xf]) {
        if (puVar2 == DAT_01edc708) goto LAB_00a34e26;
      }
      if (DAT_01edc708 == (undefined4 *)0x0) {
LAB_00a34e26:
        uVar3 = *(undefined4 *)(iVar5 + iVar4 * 4);
        if (*(int *)((int)param_1 + 0x34) == 0) {
          FUN_0104f9b0((int)this);
        }
        FUN_00503260(this,iVar1,uVar3);
        iVar5 = DAT_01edc69c;
      }
    }
    do {
      FUN_00419770(&iStack_10);
      if (DAT_01edc6a0 <= iStack_c) goto LAB_00a34e74;
    } while ((*(uint *)(*(int *)(iVar5 + iStack_c * 4) + 8) & 0x200) != 0);
  } while( true );
}


/* Also in vtable: WTInterface (slot 0) */

undefined4 __thiscall WTInterface__vfunc_0(void *this,int param_1)

{
  *(int *)((int)this + 0x10) = param_1;
  if (*(int *)((int)this + 0x3c) == 0) {
    FUN_00a34ef0((int)this);
    (**(code **)(*(int *)this + 0x2c))(*(undefined4 *)((int)this + 8));
    (**(code **)(*(int *)this + 0x2c))(*(undefined4 *)((int)this + 4));
    (**(code **)(*(int *)this + 0x2c))(*(undefined4 *)((int)this + 0xc));
  }
  (**(code **)(*(int *)this + 0xc))();
  return 1;
}


/* [VTable] WTInterface virtual function #2
   VTable: vtable_WTInterface at 019220f4 */

bool __fastcall WTInterface__vfunc_2(int param_1)

{
  return *(int *)(param_1 + 0x3c) != 0;
}




/* ========== ZipFileSystem.c ========== */

/*
 * SGW.exe - ZipFileSystem (28 functions)
 */

/* [VTable] ZipFileSystem virtual function #6
   VTable: vtable_ZipFileSystem at 017fcb04 */

uint ZipFileSystem__vfunc_6(void)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167d568;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  uVar1 = FUN_004585a0((int *)&stack0x00000008);
  ExceptionList = local_c;
  return uVar1 & 0xffffff00;
}


/* [VTable] ZipFileSystem virtual function #9
   VTable: vtable_ZipFileSystem at 017fcb04
   [String discovery] References: "ZipFileSystem::posixFileOpen" */

FILE * __thiscall ZipFileSystem__vfunc_9(void *this,undefined4 param_1)

{
  undefined4 *puVar1;
  FILE *_File;
  int iStack_224;
  undefined4 *local_220;
  CHAR aCStack_21c [264];
  CHAR aCStack_114 [264];
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167c086;
  pvStack_c = ExceptionList;
  puVar1 = (undefined4 *)((int)this + 4);
  ExceptionList = &pvStack_c;
  local_220 = puVar1;
  WaitForSingleObject((HANDLE)*puVar1,0xffffffff);
  *(undefined1 *)((int)this + 8) = 1;
  iStack_4 = 0;
  GetTempPathA(0x104,aCStack_21c);
  GetTempFileNameA(aCStack_21c,"bwt",0,aCStack_114);
  _File = fopen(aCStack_114,"w+bD");
  if (_File == (FILE *)0x0) {
    _00___FRawStaticIndexBuffer__vfunc_0();
    iStack_4 = 0xffffffff;
    *(undefined1 *)((int)this + 8) = 0;
    ReleaseMutex((HANDLE)*puVar1);
    _File = (FILE *)0x0;
  }
  else {
    (**(code **)(*(int *)this + 0x2c))(&iStack_224,param_1);
    iStack_4._0_1_ = 1;
    if (iStack_224 == 0) {
      fclose(_File);
      iStack_4 = (uint)iStack_4._1_3_ << 8;
      FUN_004585a0(&iStack_224);
      iStack_4 = 0xffffffff;
      *(undefined1 *)((int)this + 8) = 0;
      ReleaseMutex((HANDLE)*puVar1);
      _File = (FILE *)0x0;
    }
    else {
      fwrite(*(void **)(iStack_224 + 8),1,*(size_t *)(iStack_224 + 0xc),_File);
      fseek(_File,0,0);
      iStack_4 = (uint)iStack_4._1_3_ << 8;
      FUN_004585a0(&iStack_224);
      iStack_4 = 0xffffffff;
      *(undefined1 *)((int)this + 8) = 0;
      ReleaseMutex((HANDLE)*puVar1);
    }
  }
  ExceptionList = pvStack_c;
  return _File;
}


/* [VTable] ZipFileSystem virtual function #4
   VTable: vtable_ZipFileSystem at 017fcb04
   [String discovery] References: "ZipFileSystem::readFile" */

undefined4 __thiscall ZipFileSystem__vfunc_4(void *this,undefined4 param_1,uint param_2)

{
  undefined4 *puVar1;
  uint uVar2;
  char local_31 [5];
  undefined4 *puStack_2c;
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  void *local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167c0e9;
  pvStack_c = ExceptionList;
  local_31[1] = '\0';
  local_31[2] = '\0';
  local_31[3] = '\0';
  local_31[4] = '\0';
  local_10 = 0xf;
  local_31[0] = '\0';
  local_14 = (void *)0x0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_24,local_31);
  uVar2 = std::char_traits<char>::length("ZipFileSystem::readFile");
  FUN_004377d0(auStack_28,"ZipFileSystem::readFile",uVar2);
  uVar2 = param_2;
  uStack_4 = 1;
  WinFileSystem__unknown_00458290(param_2);
  uStack_4 = uStack_4 & 0xffffff00;
  if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 0xf;
  param_2 = param_2 & 0xffffff00;
  local_14 = (void *)0x0;
  std::char_traits<char>::assign((char *)local_24,(char *)&param_2);
  puVar1 = (undefined4 *)((int)this + 4);
  puStack_2c = puVar1;
  WaitForSingleObject((HANDLE)*puVar1,0xffffffff);
  *(undefined1 *)((int)this + 8) = 1;
  uStack_4 = 2;
  (**(code **)(*(int *)this + 0x2c))(param_1,uVar2);
  pvStack_c = (void *)((uint)pvStack_c & 0xffffff00);
  *(undefined1 *)((int)this + 8) = 0;
  ReleaseMutex((HANDLE)*puVar1);
  ExceptionList = local_14;
  return param_1;
}


/* WARNING: Type propagation algorithm not settling */
/* [VTable] ZipFileSystem virtual function #11
   VTable: vtable_ZipFileSystem at 017fcb04
   [String discovery] References: "ZipFileSystem::readFile" */

undefined4 * __thiscall
ZipFileSystem__vfunc_11(void *this,undefined4 param_1,void *param_2,undefined4 *param_3,int param_4)

{
  undefined1 *puVar1;
  char *******pppppppcVar2;
  char *******pppppppcVar3;
  char *******pppppppcVar4;
  void *pvVar5;
  int iVar6;
  size_t sVar7;
  size_t sVar8;
  void *pvVar9;
  undefined4 *puVar10;
  int iVar11;
  char *pcVar12;
  char local_a1 [5];
  undefined4 local_9c;
  int aiStack_98 [2];
  char *******local_90;
  undefined1 auStack_8c [4];
  char acStack_88 [8];
  int local_80;
  uint local_7c;
  undefined4 uStack_78;
  uint uStack_74;
  undefined1 *puStack_70;
  int iStack_6c;
  undefined1 *puStack_68;
  undefined1 *puStack_64;
  int iStack_60;
  int aiStack_5c [2];
  short sStack_54;
  size_t sStack_4a;
  undefined4 uStack_46;
  ushort uStack_42;
  ushort uStack_40;
  void *pvStack_3c;
  size_t asStack_38 [2];
  void *pvStack_30;
  size_t sStack_2c;
  undefined4 uStack_28;
  uint uStack_24;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  void *pvStack_c;
  undefined1 *puStack_8;
  void *pvStack_4;
  
  pvStack_4 = (void *)0xffffffff;
  puStack_8 = &LAB_0167c1ec;
  pvStack_c = ExceptionList;
  local_9c = 0;
  local_7c = 0xf;
  local_a1[0] = '\0';
  local_80 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)&local_90,local_a1);
  FUN_00437710(aiStack_98 + 1,param_2,0,0xffffffff);
  pvStack_4 = (void *)0x1;
  pppppppcVar2 = local_90;
  if (local_7c < 0x10) {
    pppppppcVar2 = (char *******)&local_90;
  }
  pppppppcVar2 = (char *******)((int)pppppppcVar2 + local_80);
  if (pppppppcVar2 == (char *******)0x0) {
LAB_004637e8:
    _invalid_parameter_noinfo();
  }
  else {
    pppppppcVar4 = local_90;
    if (local_7c < 0x10) {
      pppppppcVar4 = (char *******)&local_90;
    }
    if (pppppppcVar2 < pppppppcVar4) goto LAB_004637e8;
    pppppppcVar4 = local_90;
    if (local_7c < 0x10) {
      pppppppcVar4 = (char *******)&local_90;
    }
    if ((char *******)((int)pppppppcVar4 + local_80) < pppppppcVar2) goto LAB_004637e8;
  }
  if (local_7c < 0x10) {
    pppppppcVar4 = (char *******)&local_90;
LAB_00463809:
    pppppppcVar3 = local_90;
    if (local_7c < 0x10) {
      pppppppcVar3 = (char *******)&local_90;
    }
    if (pppppppcVar3 <= pppppppcVar4) {
      pppppppcVar3 = local_90;
      if (local_7c < 0x10) {
        pppppppcVar3 = (char *******)&local_90;
      }
      if (pppppppcVar4 <= (char *******)((int)pppppppcVar3 + local_80)) goto joined_r0x0046383b;
    }
  }
  else {
    pppppppcVar4 = local_90;
    if (local_90 != (char *******)0x0) goto LAB_00463809;
  }
  _invalid_parameter_noinfo();
joined_r0x0046383b:
  for (; pppppppcVar4 != pppppppcVar2; pppppppcVar4 = (char *******)((int)pppppppcVar4 + 1)) {
    if (*(char *)pppppppcVar4 == '\\') {
      *(char *)pppppppcVar4 = '/';
    }
  }
  if (local_7c < 0x10) {
    local_90 = (char *******)&local_90;
  }
  if (*(char *)local_90 == '/') {
    ZipFileSystem__unknown_0042e580(aiStack_98 + 1,(undefined4 *)0x0,1);
  }
  pvVar5 = (void *)ZipFileSystem__unknown_00463540
                             ((void *)((int)&uStack_46 + 2),(uint)(aiStack_98 + 1));
  FUN_0158ea90((undefined1 *)((int)this + 0x28),&puStack_70,pvVar5);
  if (0xf < uStack_24) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_24 = 0xf;
  local_9c = local_9c & 0xffffff;
  uStack_28 = 0;
  std::char_traits<char>::assign((char *)asStack_38,(char *)((int)&local_9c + 3));
  puVar1 = puStack_70;
  aiStack_98[0] = 0;
  iStack_60 = *(int *)((int)this + 0x2c);
  if ((puStack_70 == (undefined1 *)0x0) || (puStack_70 != (undefined1 *)((int)this + 0x28))) {
    _invalid_parameter_noinfo();
  }
  if (iStack_6c == iStack_60) {
    *param_3 = 0;
    aiStack_98[1] = 1;
    FUN_004585a0(aiStack_98);
    if (0xf < uStack_74) {
                    /* WARNING: Subroutine does not return */
      scalable_free();
    }
    uStack_74 = 0xf;
    local_9c = local_9c & 0xffffff;
    uStack_78 = 0;
    std::char_traits<char>::assign(acStack_88,(char *)((int)&local_9c + 3));
  }
  else {
    if (puVar1 == (undefined1 *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (iStack_6c == *(int *)(puVar1 + 4)) {
      _invalid_parameter_noinfo();
    }
    iVar6 = fseek(*(FILE **)((int)this + 0x40),*(long *)(iStack_6c + 0x28),0);
    if (iVar6 == 0) {
      sVar7 = fread(aiStack_5c,0x1e,1,*(FILE **)((int)this + 0x40));
      if (sVar7 == 1) {
        if (aiStack_5c[0] == 0x4034b50) {
          iVar6 = fseek(*(FILE **)((int)this + 0x40),(uint)uStack_40 + (uint)uStack_42,1);
          if (iVar6 == 0) {
            if ((sStack_54 == 0) || (sStack_54 == 8)) {
              pvVar5 = (void *)FUN_00418e30(sStack_4a);
              if (pvVar5 != (void *)0x0) {
                sVar8 = fread(pvVar5,1,sStack_4a,*(FILE **)((int)this + 0x40));
                sVar7 = uStack_46;
                if (sVar8 != sStack_4a) {
                  if (*(uint *)(param_4 + 0x18) < 0x10) {
                    iVar6 = param_4 + 4;
                  }
                  else {
                    iVar6 = *(int *)(param_4 + 4);
                  }
                  pcVar12 = "ZipFileSystem::readFile Data read error (%s in %s)\n";
                  _00___FRawStaticIndexBuffer__vfunc_0();
                    /* WARNING: Subroutine does not return */
                  scalable_free(pvVar5,pcVar12,iVar6);
                }
                if (sStack_54 == 8) {
                  memset(&pvStack_3c,0,0x38);
                  pvVar9 = (void *)FUN_00418e30(sVar7);
                  if (pvVar9 == (void *)0x0) {
                    if (*(uint *)(param_4 + 0x18) < 0x10) {
                      iVar6 = param_4 + 4;
                    }
                    else {
                      iVar6 = *(int *)(param_4 + 4);
                    }
                    pcVar12 = "ZipFileSystem::readFile Failed to alloc data buffer (%s in %s)\n";
                    _00___FRawStaticIndexBuffer__vfunc_0();
                    /* WARNING: Subroutine does not return */
                    scalable_free(pvVar5,pcVar12,iVar6);
                  }
                  iVar6 = FUN_012a1bb0((int)&pvStack_3c,0xfffffff1,"1.2.1",0x38);
                  if (iVar6 != 0) {
                    if (*(uint *)(param_4 + 0x18) < 0x10) {
                      iVar6 = param_4 + 4;
                    }
                    else {
                      iVar6 = *(int *)(param_4 + 4);
                    }
                    pcVar12 = "ZipFileSystem::readFile inflateInit2 failed (%s in %s)\n";
                    _00___FRawStaticIndexBuffer__vfunc_0();
                    /* WARNING: Subroutine does not return */
                    scalable_free(pvVar5,pcVar12,iVar6);
                  }
                  uStack_1c = 0;
                  uStack_18 = 0;
                  asStack_38[0] = sStack_4a;
                  sStack_2c = sVar7;
                  pvStack_3c = pvVar5;
                  pvStack_30 = pvVar9;
                  iVar6 = FUN_012a1dc0((int *)&pvStack_3c,4);
                  if (iVar6 != 1) {
                    if (*(uint *)(param_4 + 0x18) < 0x10) {
                      iVar11 = param_4 + 4;
                    }
                    else {
                      iVar11 = *(int *)(param_4 + 4);
                    }
                    pcVar12 = "ZipFileSystem::readFile Decompression error %d (%s in %s)\n";
                    _00___FRawStaticIndexBuffer__vfunc_0();
                    /* WARNING: Subroutine does not return */
                    scalable_free(pvVar5,pcVar12,iVar6,iVar11);
                  }
                  FUN_012a33f0((int)&pvStack_3c);
                  puStack_68 = (undefined1 *)FUN_00418e30(0x14);
                  if (puStack_68 == (undefined1 *)0x0) {
                    puVar10 = (undefined4 *)0x0;
                  }
                  else {
                    puStack_70 = &stack0xffffff50;
                    puStack_64 = &stack0xffffff50;
                    puVar10 = FUN_00472290(puStack_68,pvVar9,uStack_46,0);
                  }
                  FUN_0046bcb0(&local_90,(int)puVar10,'\0');
                }
                else {
                  puStack_70 = (undefined1 *)FUN_00418e30(0x14);
                  if (puStack_70 == (undefined1 *)0x0) {
                    puVar10 = (undefined4 *)0x0;
                  }
                  else {
                    puStack_64 = &stack0xffffff50;
                    puStack_68 = &stack0xffffff50;
                    puVar10 = FUN_00472290(puStack_70,pvVar5,sStack_4a,0);
                  }
                  FUN_0046bcb0(&local_90,(int)puVar10,'\0');
                }
                FUN_0046bd00(aiStack_98,(int *)&local_90);
                FUN_004585a0((int *)&local_90);
                    /* WARNING: Subroutine does not return */
                scalable_free();
              }
              _00___FRawStaticIndexBuffer__vfunc_0();
              *param_3 = 0;
            }
            else {
              _00___FRawStaticIndexBuffer__vfunc_0();
              *param_3 = 0;
            }
          }
          else {
            _00___FRawStaticIndexBuffer__vfunc_0();
            *param_3 = 0;
          }
        }
        else {
          _00___FRawStaticIndexBuffer__vfunc_0();
          *param_3 = 0;
        }
      }
      else {
        _00___FRawStaticIndexBuffer__vfunc_0();
        *param_3 = 0;
      }
    }
    else {
      _00___FRawStaticIndexBuffer__vfunc_0();
      *param_3 = 0;
    }
    aiStack_98[1] = 1;
    FUN_004585a0(aiStack_98);
    ZipFileSystem__unknown_00433ed0((uint)auStack_8c);
  }
  ExceptionList = pvStack_4;
  return param_3;
}


/* [VTable] ZipFileSystem virtual function #2
   VTable: vtable_ZipFileSystem at 017fcb04 */

void * ZipFileSystem__vfunc_2(void *param_1,uint param_2,undefined4 param_3)

{
  uint uVar1;
  char local_2c [32];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167c259;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_2c,"%s %s %02d %02d:%02d:%02d %d",&DAT_017fca88,
          (&PTR_PTR_01dae13c)[param_2 >> 0x15 & 0xf],param_2 >> 0x10 & 0x1f,
          (param_2 & 0xffff) >> 0xb,(param_2 & 0xffff) >> 5 & 0xf,(param_2 & 0x1f) * 2,
          (param_2 >> 0x19) + 0x7bc);
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  param_2 = param_2 & 0xffffff00;
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<char>::assign((char *)((int)param_1 + 4),(char *)&param_2);
  uVar1 = std::char_traits<char>::length(local_2c);
  FUN_004377d0(param_1,local_2c,uVar1);
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] ZipFileSystem virtual function #3
   VTable: vtable_ZipFileSystem at 017fcb04
   [String discovery] References: "ZipFileSystem::readDirectory" */

void * __thiscall
ZipFileSystem__vfunc_3(void *this,undefined4 param_1,void *param_2,void *param_3,char param_4)

{
  int iVar1;
  uint uVar2;
  char ****ppppcVar3;
  char ****ppppcVar4;
  char ****ppppcVar5;
  void *pvVar6;
  void *this_00;
  void *pvVar7;
  uint uVar8;
  void **ppvVar9;
  HANDLE hMutex;
  char local_5d [5];
  undefined4 *puStack_58;
  void *local_54;
  undefined4 *puStack_50;
  void *pvStack_4c;
  uint uStack_48;
  uint uStack_44;
  char ***apppcStack_40 [2];
  undefined4 auStack_38 [2];
  int iStack_30;
  uint uStack_2c;
  undefined4 uStack_28;
  uint local_24 [2];
  undefined4 auStack_1c [2];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  void *pvStack_4;
  
  pvStack_4 = (void *)0xffffffff;
  puStack_8 = &LAB_0167c2f1;
  pvStack_c = ExceptionList;
  local_5d[1] = '\0';
  local_5d[2] = '\0';
  local_5d[3] = '\0';
  local_5d[4] = '\0';
  local_10 = 0xf;
  local_5d[0] = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  local_54 = this;
  std::char_traits<char>::assign((char *)local_24,local_5d);
  uVar2 = std::char_traits<char>::length("ZipFileSystem::readDirectory");
  FUN_004377d0(&uStack_28,"ZipFileSystem::readDirectory",uVar2);
  pvVar7 = param_2;
  pvStack_4 = (void *)0x1;
  WinFileSystem__unknown_00458290((int)param_2);
  pvStack_4 = (void *)((uint)pvStack_4 & 0xffffff00);
  if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 0xf;
  param_2 = (void *)((uint)param_2 & 0xffffff00);
  local_14 = 0;
  std::char_traits<char>::assign((char *)local_24,(char *)&param_2);
  puStack_58 = (undefined4 *)((int)this + 4);
  WaitForSingleObject((HANDLE)*puStack_58,0xffffffff);
  *(undefined1 *)((int)this + 8) = 1;
  pvStack_4 = (void *)0x2;
  uStack_2c = 0xf;
  param_2 = (void *)((uint)param_2 & 0xffffff00);
  iStack_30 = 0;
  std::char_traits<char>::assign((char *)apppcStack_40,(char *)&param_2);
  FUN_00437710(&uStack_44,pvVar7,0,0xffffffff);
  pvStack_4 = (void *)CONCAT31(pvStack_4._1_3_,3);
  ppppcVar5 = (char ****)apppcStack_40[0];
  if (uStack_2c < 0x10) {
    ppppcVar5 = apppcStack_40;
  }
  ppppcVar5 = (char ****)((int)ppppcVar5 + iStack_30);
  if (ppppcVar5 == (char ****)0x0) {
LAB_00464cba:
    _invalid_parameter_noinfo();
  }
  else {
    ppppcVar4 = (char ****)apppcStack_40[0];
    if (uStack_2c < 0x10) {
      ppppcVar4 = apppcStack_40;
    }
    if (ppppcVar5 < ppppcVar4) goto LAB_00464cba;
    ppppcVar4 = (char ****)apppcStack_40[0];
    if (uStack_2c < 0x10) {
      ppppcVar4 = apppcStack_40;
    }
    if ((char ****)((int)ppppcVar4 + iStack_30) < ppppcVar5) goto LAB_00464cba;
  }
  if (uStack_2c < 0x10) {
    ppppcVar4 = apppcStack_40;
LAB_00464cdb:
    ppppcVar3 = (char ****)apppcStack_40[0];
    if (uStack_2c < 0x10) {
      ppppcVar3 = apppcStack_40;
    }
    if (ppppcVar3 <= ppppcVar4) {
      ppppcVar3 = (char ****)apppcStack_40[0];
      if (uStack_2c < 0x10) {
        ppppcVar3 = apppcStack_40;
      }
      if (ppppcVar4 <= (char ****)((int)ppppcVar3 + iStack_30)) goto joined_r0x00464d0d;
    }
  }
  else {
    ppppcVar4 = (char ****)apppcStack_40[0];
    if ((char ****)apppcStack_40[0] != (char ****)0x0) goto LAB_00464cdb;
  }
  _invalid_parameter_noinfo();
joined_r0x00464d0d:
  for (; ppppcVar4 != ppppcVar5; ppppcVar4 = (char ****)((int)ppppcVar4 + 1)) {
    if (*(char *)ppppcVar4 == '\\') {
      *(char *)ppppcVar4 = '/';
    }
  }
  ppppcVar5 = (char ****)apppcStack_40[0];
  if (uStack_2c < 0x10) {
    ppppcVar5 = apppcStack_40;
  }
  if (*(char *)ppppcVar5 == '/') {
    ZipFileSystem__unknown_0042e580(&uStack_44,(undefined4 *)0x0,1);
  }
  pvVar6 = (void *)ZipFileSystem__unknown_00463540(&uStack_28,(uint)&uStack_44);
  pvVar7 = pvStack_4c;
  this_00 = (void *)((int)pvStack_4c + 0x34);
  ZipFileSystem__unknown_004634c0(this_00,&pvStack_4c,pvVar6);
  if ((undefined1 *)0xf < puStack_8) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_1c[0]);
  }
  puStack_8 = (undefined1 *)0xf;
  param_4 = '\0';
  pvStack_c = (void *)0x0;
  std::char_traits<char>::assign((char *)auStack_1c,&param_4);
  iVar1 = *(int *)((int)pvVar7 + 0x38);
  if ((pvStack_4c == (void *)0x0) || (pvStack_4c != this_00)) {
    _invalid_parameter_noinfo();
  }
  uVar2 = uStack_48;
  pvVar7 = param_3;
  if (uStack_48 == iVar1) {
    uStack_48 = 0;
    uStack_44 = 0;
    apppcStack_40[0] = (char ***)0x0;
    FUN_00462910(param_3,&pvStack_4c);
    local_54 = (void *)0x1;
    if (uStack_48 != 0) {
      ppvVar9 = &pvStack_4c;
      uVar2 = uStack_48;
      uVar8 = uStack_44;
      pvVar7 = param_3;
      WinFileSystem__unknown_00459570(uStack_48,uStack_44);
                    /* WARNING: Subroutine does not return */
      scalable_free(uStack_48,uVar2,uVar8,ppvVar9,pvVar7);
    }
    uStack_48 = 0;
    uStack_44 = 0;
    apppcStack_40[0] = (char ***)0x0;
    if (0xf < local_24[0]) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_38[0]);
    }
    local_24[0] = 0xf;
    param_4 = '\0';
    uStack_28 = 0;
    std::char_traits<char>::assign((char *)auStack_38,&param_4);
    *(undefined1 *)(puStack_50 + 1) = 0;
    hMutex = (HANDLE)*puStack_50;
  }
  else {
    if (pvStack_4c == (void *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (uVar2 == *(int *)((int)pvStack_4c + 4)) {
      _invalid_parameter_noinfo();
    }
    pvVar7 = param_3;
    FUN_00462910(param_3,(void *)(uVar2 + 0x28));
    local_54 = (void *)0x1;
    if (0xf < local_24[0]) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_38[0]);
    }
    local_24[0] = 0xf;
    param_4 = '\0';
    uStack_28 = 0;
    std::char_traits<char>::assign((char *)auStack_38,&param_4);
    hMutex = (HANDLE)*puStack_50;
    *(undefined1 *)(puStack_50 + 1) = 0;
  }
  ReleaseMutex(hMutex);
  ExceptionList = pvStack_4;
  return pvVar7;
}


/* Also in vtable: ZipFileSystem (slot 0) */

undefined4 * __thiscall ZipFileSystem__vfunc_0(void *this,byte param_1)

{
  FUN_00465b90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] ZipFileSystem virtual function #10
   VTable: vtable_ZipFileSystem at 017fcb04 */

void __fastcall ZipFileSystem__vfunc_10(int param_1)

{
  void *this;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this = (void *)scalable_malloc(0x44);
  if (this == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_004667b0(this,(void *)(param_1 + 0xc));
  ExceptionList = local_c;
  return;
}


/* Also in vtable: CZipException (slot 0) */

exception * __thiscall CZipException__vfunc_0(void *this,byte param_1)

{
  FUN_013945e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CZipAbstractFile virtual function #13
   VTable: vtable_CZipAbstractFile at 017fed84 */

undefined4 * __thiscall CZipAbstractFile__vfunc_13(void *this,byte param_1)

{
  *(undefined ***)this = CZipAbstractFile::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CZipMemFile virtual function #5
   VTable: vtable_CZipMemFile at 017fedc0 */

undefined4 __fastcall CZipMemFile__vfunc_5(int param_1)

{
  return *(undefined4 *)(param_1 + 0x10);
}


/* [VTable] CZipMemFile virtual function #12
   VTable: vtable_CZipMemFile at 017fedc0 */

bool __fastcall CZipMemFile__vfunc_12(int param_1)

{
  return *(int *)(param_1 + 0x14) == 0;
}


/* [VTable] CZipMemFile virtual function #3
   VTable: vtable_CZipMemFile at 017fedc0 */

undefined4 __fastcall CZipMemFile__vfunc_3(int param_1)

{
  return *(undefined4 *)(param_1 + 8);
}


/* [VTable] CZipMemFile virtual function #1
   VTable: vtable_CZipMemFile at 017fedc0 */

void __fastcall CZipMemFile__vfunc_1(int param_1)

{
  if ((*(char *)(param_1 + 0x18) != '\0') && (*(void **)(param_1 + 0x14) != (void *)0x0)) {
    free(*(void **)(param_1 + 0x14));
    *(undefined4 *)(param_1 + 0x14) = 0;
  }
  *(undefined4 *)(param_1 + 8) = 0;
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 0x10) = 0;
  *(undefined4 *)(param_1 + 0xc) = 0;
  *(undefined4 *)(param_1 + 0x14) = 0;
  return;
}


/* [VTable] CZipMemFile virtual function #13
   VTable: vtable_CZipMemFile at 017fedc0 */

undefined4 * __thiscall CZipMemFile__vfunc_13(void *this,byte param_1)

{
  FUN_00478d50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CZipMemFile virtual function #9
   VTable: vtable_CZipMemFile at 017fedc0 */

void * CZipMemFile__vfunc_9(void *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167e879;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0047ace0(param_1,(wchar_t *)&PTR_017f8e10);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] CZipMemFile virtual function #10
   VTable: vtable_CZipMemFile at 017fedc0 */

size_t __thiscall CZipMemFile__vfunc_10(void *this,void *param_1,size_t param_2)

{
  uint uVar1;
  uint uVar2;
  
  uVar1 = *(uint *)((int)this + 8);
  uVar2 = *(uint *)((int)this + 0x10);
  if (uVar2 <= uVar1) {
    return 0;
  }
  if (uVar2 < uVar1 + param_2) {
    param_2 = uVar2 - uVar1;
  }
  memcpy(param_1,(void *)(*(int *)((int)this + 0x14) + uVar1),param_2);
  *(int *)((int)this + 8) = *(int *)((int)this + 8) + param_2;
  return param_2;
}


/* [VTable] CZipMemFile virtual function #6
   VTable: vtable_CZipMemFile at 017fedc0 */

void __thiscall CZipMemFile__vfunc_6(void *this,uint param_1)

{
  if (*(uint *)((int)this + 0xc) < param_1) {
    CZipMemFile__unknown_01395850(this,param_1);
    *(uint *)((int)this + 0x10) = param_1;
    return;
  }
  *(uint *)((int)this + 8) = param_1;
  *(uint *)((int)this + 0x10) = param_1;
  return;
}


/* [VTable] CZipMemFile virtual function #11
   VTable: vtable_CZipMemFile at 017fedc0 */

void __thiscall CZipMemFile__vfunc_11(void *this,void *param_1,size_t param_2)

{
  uint uVar1;
  
  if (param_2 != 0) {
    uVar1 = *(int *)((int)this + 8) + param_2;
    if (*(uint *)((int)this + 0xc) < uVar1) {
      CZipMemFile__unknown_01395850(this,uVar1);
    }
    memcpy((void *)(*(int *)((int)this + 0x14) + *(int *)((int)this + 8)),param_1,param_2);
    *(int *)((int)this + 8) = *(int *)((int)this + 8) + param_2;
    if (*(uint *)((int)this + 0x10) < *(uint *)((int)this + 8)) {
      *(uint *)((int)this + 0x10) = *(uint *)((int)this + 8);
    }
  }
  return;
}


/* [VTable] CZipMemFile virtual function #4
   VTable: vtable_CZipMemFile at 017fedc0 */

undefined8 __thiscall CZipMemFile__vfunc_4(void *this,uint param_1,uint param_2,int param_3)

{
  uint *puVar1;
  int iVar2;
  void *pvVar3;
  void *extraout_EDX;
  uint uVar4;
  uint uVar5;
  bool bVar6;
  undefined8 uVar7;
  
  uVar7 = CONCAT44(this,param_3);
  uVar4 = 0;
  uVar5 = *(uint *)((int)this + 8);
  pvVar3 = this;
  if (param_3 == 0) {
    if (((int)param_2 < 1) && ((int)param_2 < 0)) {
      uVar7 = CZipMemFile__unknown_0047ad90(0x1f9,(wchar_t *)0x0);
      uVar4 = param_2;
      uVar5 = param_1;
      goto LAB_0139596e;
    }
  }
  else {
LAB_0139596e:
    pvVar3 = (void *)((ulonglong)uVar7 >> 0x20);
    if ((int)uVar7 == 1) {
      if (((int)param_2 < 1) && ((int)param_2 < 0)) {
        iVar2 = param_2 + (param_1 != 0);
        if ((uVar4 <= (uint)-iVar2) &&
           ((-uVar4 != iVar2 || (uVar5 <= -param_1 && -uVar5 != param_1)))) {
          uVar7 = CZipMemFile__unknown_0047ad90(0x1f9,(wchar_t *)0x0);
          goto LAB_013959a6;
        }
      }
    }
    else {
LAB_013959a6:
      pvVar3 = (void *)((ulonglong)uVar7 >> 0x20);
      if ((int)uVar7 != 2) goto LAB_01395a23;
      if (((int)param_2 < 1) && ((int)param_2 < 0)) {
        puVar1 = (uint *)((int)pvVar3 + 0x10);
        if ((param_2 + (param_1 != 0) != 0) ||
           (pvVar3 = this, *puVar1 <= -param_1 && -*puVar1 != param_1)) {
          CZipMemFile__unknown_0047ad90(0x1f9,(wchar_t *)0x0);
          pvVar3 = this;
        }
      }
      uVar5 = *(uint *)((int)pvVar3 + 0x10);
      uVar4 = 0;
    }
    bVar6 = CARRY4(uVar5,param_1);
    param_1 = uVar5 + param_1;
    param_2 = uVar4 + param_2 + (uint)bVar6;
  }
  if (param_2 != 0) {
    CZipMemFile__unknown_0047ad90(0x1f9,(wchar_t *)0x0);
    pvVar3 = extraout_EDX;
  }
  if ((param_2 != 0) || (*(uint *)((int)pvVar3 + 0x10) < param_1)) {
    CZipMemFile__unknown_01395850(pvVar3,param_1);
    pvVar3 = this;
  }
  *(uint *)((int)pvVar3 + 8) = param_1;
  uVar4 = param_2;
  uVar5 = param_1;
LAB_01395a23:
  return CONCAT44(uVar4,uVar5);
}


undefined4 * __thiscall CZipPathComponent__vfunc_0(void *this,byte param_1)

{
  FUN_0139e480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CZipArchive__vfunc_0(void *this,byte param_1)

{
  FUN_01399c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CZipFileHeader__vfunc_0(void *this,byte param_1)

{
  FCurveEdInterface__unknown_0139c9e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


uint __thiscall CZipFile__vfunc_0(void *this,wchar_t *param_1,uint param_2,char param_3)

{
  bool bVar1;
  char cVar2;
  int iVar3;
  undefined4 extraout_EAX;
  uint uVar4;
  basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_std::allocator<wchar_t>_>
  abStack_28 [28];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0177d3f8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  cVar2 = (**(code **)(*(int *)this + 0x30))(DAT_01e7f9a0 ^ (uint)&stack0xffffffcc);
  if (cVar2 == '\0') {
    (**(code **)(*(int *)this + 4))();
  }
  bVar1 = false;
  uVar4 = 0x8000;
  if ((param_2 & 0x20) != 0) {
    uVar4 = 0x8100;
  }
  if (((byte)param_2 & 3) == 3) {
    uVar4 = uVar4 | 2;
  }
  else if ((param_2 & 1) == 0) {
    if ((param_2 & 2) != 0) {
      uVar4 = uVar4 | 1;
    }
  }
  else {
    bVar1 = true;
  }
  if (((param_2 & 0x40) == 0) && (!bVar1)) {
    uVar4 = uVar4 | 0x200;
  }
  iVar3 = FUN_0139d680(param_1,uVar4,param_2 & 0x1c);
  *(int *)((int)this + 4) = iVar3;
  if (iVar3 == -1) {
    uVar4 = 0xffffffff;
    if (param_3 != '\0') {
      uVar4 = FUN_0139e250((int)this);
    }
    ExceptionList = local_c;
    return uVar4 & 0xffffff00;
  }
  FUN_0047ace0(abStack_28,param_1);
  uStack_4 = 0;
  std::basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_std::allocator<wchar_t>_>::
  operator=((basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_std::allocator<wchar_t>_> *
            )((int)this + 8),abStack_28);
  uStack_4 = 0xffffffff;
  std::basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_std::allocator<wchar_t>_>::
  ~basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_std::allocator<wchar_t>_>(abStack_28)
  ;
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)extraout_EAX >> 8),1);
}


undefined4 * __thiscall CZipCentralDir__vfunc_0(void *this,byte param_1)

{
  FUN_013a04f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CZipAutoBuffer__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CZipAutoBuffer::vftable;
  if (*(int *)((int)this + 4) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(int *)((int)this + 4));
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall CZipStorage__vfunc_0(void *this,byte param_1)

{
  FUN_013a1620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


bool __thiscall CZipCrc32Cryptograph__vfunc_0(void *this,int param_1,int param_2,void *param_3)

{
  byte bVar1;
  uint uVar2;
  uint uVar3;
  undefined **ppuVar4;
  byte bVar5;
  int iVar6;
  undefined4 local_18;
  int local_14;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0177d588;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_013a32f0(this,param_1);
  iVar6 = 0;
  FUN_013a1560(&local_18,0xc,'\0');
  local_4 = 0;
  FUN_013a2bc0(param_3,local_14,0xc,'\0');
  do {
    uVar2 = *(uint *)((int)this + 4);
    uVar3 = *(uint *)((int)this + 0xc) & 0xfffd | 2;
    bVar5 = (byte)((uVar3 ^ 1) * uVar3 >> 8) ^ *(byte *)(local_14 + iVar6);
    ppuVar4 = FUN_013a41c0();
    uVar3 = (uint)ppuVar4[(bVar5 ^ uVar2) & 0xff] ^ uVar2 >> 8;
    uVar2 = *(uint *)((int)this + 0xc);
    *(uint *)((int)this + 4) = uVar3;
    uVar3 = ((uVar3 & 0xff) + *(int *)((int)this + 8)) * 0x8088405 + 1;
    *(uint *)((int)this + 8) = uVar3;
    ppuVar4 = FUN_013a41c0();
    iVar6 = iVar6 + 1;
    *(uint *)((int)this + 0xc) = uVar2 >> 8 ^ (uint)ppuVar4[(uVar3 >> 0x18 ^ uVar2) & 0xff];
  } while (iVar6 < 0xc);
  if ((*(byte *)(param_2 + 8) >> 3 & 1) == 0) {
    bVar1 = *(byte *)(param_2 + 0x13);
  }
  else {
    bVar1 = *(byte *)(param_2 + 0xd);
  }
  local_4 = 0xffffffff;
  FCurveEdInterface__unknown_013a1590(&local_18);
  ExceptionList = local_c;
  return bVar1 == bVar5;
}




/* ========== charNode.c ========== */

/*
 * SGW.exe - charNode (1 functions)
 */


/* Library Function - Single Match
    public: virtual int __thiscall charNode::length(void)const 
   
   Library: Visual Studio 2019 Debug */

int __thiscall charNode::length(charNode *this)

{
  return 1;
}





/* ========== nVidia.c ========== */

/*
 * SGW.exe - nVidia (2 functions)
 */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [String discovery] Debug string: "nv::String::setRefCount"
   String at: 01b4944c */

void nv_String_setRefCount(void)

{
  code *pcVar1;
  int iVar2;
  short unaff_SI;
  
  if (unaff_SI == -1) {
    if ((_DAT_01f15970 & 1) == 0) {
      _DAT_01f15970 = _DAT_01f15970 | 1;
      _DAT_01f1596c = _anon_EA36DAB8::Win32AssertHandler::vftable;
      FUN_012375cb((_onexit_t)&LAB_017da500);
    }
    if (DAT_01f15938 == (undefined4 *)0x0) {
      _A0xea36dab8_Win32AssertHandler__vfunc_0();
      pcVar1 = (code *)swi(3);
      (*pcVar1)();
      return;
    }
    iVar2 = (**(code **)*DAT_01f15938)
                      ("count < 0xFFFF",
                       "C:\\perforce\\eng1\\SGW\\Working\\Development\\External\\NVIDIA Texture Tools 2\\src\\src\\nvcore/StrLib.h"
                       ,0x136,"nv::String::setRefCount");
    if (iVar2 == 1) {
      pcVar1 = (code *)swi(3);
      (*pcVar1)();
      return;
    }
  }
  *(short *)(DAT_01f1594c + -2) = unaff_SI;
  return;
}


/* [String discovery] Debug string: "nv::StringBuilder::format"
   String at: 01b49474 */

void nv_StringBuilder_format(va_list param_1)

{
  code *pcVar1;
  int iVar2;
  void *pvVar3;
  char *_DstBuf;
  size_t *unaff_ESI;
  char *unaff_EDI;
  
  if (unaff_EDI == (char *)0x0) {
    iVar2 = FUN_00402730("fmt != 0",
                         "C:\\perforce\\eng1\\SGW\\Working\\Development\\External\\NVIDIA Texture Tools 2\\src\\src\\nvcore\\StrLib.cpp"
                         ,0x119,"nv::StringBuilder::format");
    if (iVar2 == 1) {
      pcVar1 = (code *)swi(3);
      (*pcVar1)();
      return;
    }
  }
  if (*unaff_ESI == 0) {
    *unaff_ESI = 0x40;
    pvVar3 = malloc(0x40);
    unaff_ESI[1] = (size_t)pvVar3;
  }
  iVar2 = _vsnprintf_s((char *)unaff_ESI[1],*unaff_ESI,0xffffffff,unaff_EDI,param_1);
  while ((iVar2 < 0 || ((int)*unaff_ESI <= iVar2))) {
    if (iVar2 < 0) {
      *unaff_ESI = *unaff_ESI * 2;
    }
    else {
      *unaff_ESI = iVar2 + 1;
    }
    _DstBuf = realloc((void *)unaff_ESI[1],*unaff_ESI);
    unaff_ESI[1] = (size_t)_DstBuf;
    iVar2 = _vsnprintf_s(_DstBuf,*unaff_ESI,0xffffffff,unaff_EDI,param_1);
  }
  return;
}




/* ========== pcharNode.c ========== */

/*
 * SGW.exe - pcharNode (4 functions)
 */


/* Library Function - Single Match
    public: __thiscall pcharNode::pcharNode<1>(char const *,int,struct StringLifeSelector<1>)
   
   Library: Visual Studio 2019 Debug */

pcharNode * __thiscall
pcharNode::pcharNode<1>(pcharNode *this,undefined4 param_1,undefined4 param_2)

{
  FUN_013ccfe0((undefined4 *)this);
  *(undefined ***)this =
       `private:_virtual_bool___thiscall_GFxFontResourceCreator::CreateResource(void*,struct_GFxResourceBindData*,class_GFxLoadStates*)const_'
       ::__l23::SharedState::vftable;
  *(undefined4 *)(this + 4) = param_2;
  *(undefined4 *)(this + 8) = param_1;
  return this;
}




/* Library Function - Single Match
    public: __thiscall pcharNode::pcharNode<1>(char const *,int,struct StringLifeSelector<1>)
   
   Library: Visual Studio 2019 Debug */

pcharNode * __thiscall
pcharNode::pcharNode<1>(pcharNode *this,undefined4 param_1,undefined4 param_2)

{
  FUN_01415150((undefined4 *)this);
  *(undefined ***)this =
       `public:_void___thiscall_GASActionBuffer::Execute(class_GASEnvironment*,int,int,class_GASValue*,class_std::garray<class_GASWithStackEntry>_const&,enum_GASActionBuffer::ExecuteType)'
       ::__l342::EnumerateOpVisitor::vftable;
  *(undefined4 *)(this + 4) = param_1;
  *(undefined4 *)(this + 8) = param_2;
  return this;
}




/* Library Function - Single Match
    public: __thiscall pcharNode::pcharNode<1>(char const *,int,struct StringLifeSelector<1>)
   
   Library: Visual Studio 2019 Debug */

pcharNode * __thiscall
pcharNode::pcharNode<1>(pcharNode *this,undefined4 param_1,undefined4 param_2)

{
  FUN_01415150((undefined4 *)this);
  *(undefined ***)this =
       `public:_virtual_class_GASString___thiscall_GASXmlNode::toString(class_GASEnvironment*)'::
       __l8::MemberVisitor::vftable;
  *(undefined4 *)(this + 4) = param_1;
  *(undefined4 *)(this + 8) = param_2;
  return this;
}




/* Library Function - Single Match
    public: __thiscall pcharNode::pcharNode<1>(char const *,int,struct StringLifeSelector<1>)
   
   Library: Visual Studio 2019 Debug */

pcharNode * __thiscall
pcharNode::pcharNode<1>(pcharNode *this,undefined4 param_1,undefined4 param_2)

{
  FUN_01415150((undefined4 *)this);
  *(undefined ***)this =
       `public:_static_void___cdecl_GASLoadVarsProto::ToString(class_GASFnCall_const&)'::__l2::
       MemberVisitor::vftable;
  *(undefined4 *)(this + 4) = param_1;
  *(undefined4 *)(this + 8) = param_2;
  return this;
}




