
void __thiscall FUN_0085829e(int param_1,int param_2)

{
  undefined4 *puVar1;
  
  *(int *)(param_1 + 4) = param_2;
  if (param_2 != 1) {
    if (param_2 == 2) {
      puVar1 = (undefined4 *)(param_1 + 0x20);
      goto LAB_008582c0;
    }
    if (param_2 != 3) {
      if (param_2 != 4) {
        return;
      }
      puVar1 = (undefined4 *)(param_1 + 8);
      goto LAB_008582c0;
    }
  }
  puVar1 = (undefined4 *)(param_1 + 0x14);
LAB_008582c0:
  *(undefined4 *)(param_1 + 0x34) = *puVar1;
  *(undefined4 *)(param_1 + 0x38) = puVar1[1];
  *(undefined4 *)(param_1 + 0x3c) = puVar1[2];
  return;
}

