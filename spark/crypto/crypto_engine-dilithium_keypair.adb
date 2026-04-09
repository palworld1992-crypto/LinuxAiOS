--  SPARK_Mode (Off): Wrapper calls external liboqs library (OQS_SIG_ml_dsa_65_keypair)
--  which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with System;

separate (Crypto_Engine)

procedure Dilithium_Keypair 
  (Public_Key  : System.Address;
   Secret_Key  : System.Address;
   Status      : out Interfaces.C.int) is

   Result : Interfaces.C.int;
begin
   -- Ensure liboqs is initialized
   Ensure_OQS_Initialized;
   
   -- Use ML-DSA-65 specific function for stability
   Result := OQS_SIG_ml_dsa_65_keypair (Public_Key, Secret_Key);
   
   Status := Result;
   return;

exception
   when others =>
      Status := -1;
      return;
end Dilithium_Keypair;