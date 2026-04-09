--  SPARK_Mode (Off): Wrapper calls external liboqs library (OQS_SIG_ml_dsa_65_sign)
--  which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with System;
with System.Address_To_Access_Conversions;

separate (Crypto_Engine)

procedure Dilithium_Sign
  (Secret_Key     : System.Address;
   Message        : System.Address;
   Message_Len    : Interfaces.C.size_t;
   Signature_Buf  : System.Address;
   Signature_Buf_Size : Interfaces.C.size_t;
   Signature_Len_Ptr : System.Address;  -- Address of size_t
   Status         : out Interfaces.C.int) is

   Actual_Sig_Len : aliased Interfaces.C.size_t := Signature_Buf_Size;
   Result         : Interfaces.C.int;
   
   -- Convert address to access
   package Size_Conv is new System.Address_To_Access_Conversions(Interfaces.C.size_t);
   Sig_Len_Ptr : constant Size_Conv.Object_Pointer := Size_Conv.To_Pointer(Signature_Len_Ptr);
begin
   -- Ensure liboqs is initialized
   Ensure_OQS_Initialized;
   
   -- Validate caller buffer (ML-DSA-65 max signature = 3309 bytes)
   if Signature_Buf = System.Null_Address
     or else Signature_Buf_Size < 3309
     or else Signature_Len_Ptr = System.Null_Address
   then
      Status := -1;
      return;
   end if;

   -- Validate secret key pointer
   if Secret_Key = System.Null_Address then
      Status := -1;
      return;
   end if;

   -- Use ML-DSA-65 specific function (no sig object needed)
   Result := OQS_SIG_ml_dsa_65_sign 
     (Signature_Buf, 
      Actual_Sig_Len'Address, 
      Message, 
      Message_Len, 
      Secret_Key);
   
   -- Copy result to caller's Signature_Len
   Sig_Len_Ptr.all := Actual_Sig_Len;
   Status := Result;

exception
   when others =>
      Status := -1;
end Dilithium_Sign;