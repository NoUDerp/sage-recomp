
void __thiscall FUN_00430d10(int param_1,uint param_2)

{
  uint uVar1;
  
  uVar1 = *(uint *)(param_1 + 4) & 1;
  if (param_2 == 0) {
    *(uint *)(param_1 + 4) = uVar1;
    return;
  }
  if (0x50 < param_2) {
    param_2 = 0x50;
  }
  if (param_2 + 0xb < 0x11) {
    *(uint *)(param_1 + 4) = uVar1 | 0x10;
    return;
  }
  *(uint *)(param_1 + 4) = param_2 + 0xb & 0xfffffff8 | uVar1;
  return;
}

