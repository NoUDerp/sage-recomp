
void FUN_00a198d0(uint param_1,uint param_2)

{
  uint uVar1;
  
  while (uVar1 = param_2, uVar1 != 0) {
    param_2 = param_1 % uVar1;
    param_1 = uVar1;
  }
  return;
}

