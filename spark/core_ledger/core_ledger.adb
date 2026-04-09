package body Core_Ledger is
   pragma SPARK_Mode (Off);
   --  Cannot prove OpenSSL FFI calls (EVP_MD_fetch, malloc) and System.Address usage
   use type Interfaces.C.int;
   use type Interfaces.C.size_t;

   Max_Blocks : constant := 1000;

   type Block_Record is record
      Prev_Hash : Hash_Array;
      Data      : Block_Data;
      Data_Len  : Interfaces.C.size_t;
      Hash      : Hash_Array;
      Timestamp : Interfaces.C.unsigned_long;
   end record;
   pragma Pack (Block_Record);

   Blocks : array (1 .. Max_Blocks) of Block_Record;
   Count : Interfaces.C.size_t := 0;
   pragma Atomic (Count);

   function SHA256 (Data : System.Address; Len : Interfaces.C.size_t) return Hash_Array is
      use Interfaces.C.Strings;
      procedure C_Free (Ptr : System.Address)
        with Import, Convention => C, External_Name => "free";
      function C_Malloc (Size : Interfaces.C.size_t) return System.Address
        with Import, Convention => C, External_Name => "malloc";
      function EVP_MD_fetch (libctx : System.Address; name : chars_ptr; propq : System.Address) return System.Address
        with Import, Convention => C, External_Name => "EVP_MD_fetch";
      function EVP_MD_CTX_new return System.Address
        with Import, Convention => C, External_Name => "EVP_MD_CTX_new";
      procedure EVP_MD_CTX_free (ctx : System.Address)
        with Import, Convention => C, External_Name => "EVP_MD_CTX_free";
      function EVP_DigestInit_ex2 (ctx : System.Address; md : System.Address; params : System.Address) return Interfaces.C.int
        with Import, Convention => C, External_Name => "EVP_DigestInit_ex2";
      function EVP_DigestUpdate (ctx : System.Address; data : System.Address; cnt : Interfaces.C.size_t) return Interfaces.C.int
        with Import, Convention => C, External_Name => "EVP_DigestUpdate";
      function EVP_DigestFinal_ex (ctx : System.Address; md : System.Address; s : System.Address) return Interfaces.C.int
        with Import, Convention => C, External_Name => "EVP_DigestFinal_ex";

      MD_Name : constant String := "SHA256" & ASCII.NUL;
      C_Name : chars_ptr := New_String (MD_Name);
      MD : System.Address;
      MD_Ctx : System.Address;
      Digest : Hash_Array;
      Digest_Len : aliased Interfaces.C.size_t;
   begin
      MD := EVP_MD_fetch (System.Null_Address, C_Name, System.Null_Address);

      MD_Ctx := EVP_MD_CTX_new;

      if EVP_DigestInit_ex2 (MD_Ctx, MD, System.Null_Address) /= 0 and then
         EVP_DigestUpdate (MD_Ctx, Data, Len) /= 0 and then
         EVP_DigestFinal_ex (MD_Ctx, Digest'Address, Digest_Len'Access) /= 0
      then
         null;
      end if;

      EVP_MD_CTX_free (MD_Ctx);
      Free (C_Name);

      return Digest;
   end SHA256;

   procedure Add_Block
     (Prev_Hash : Hash_Array;
      Data      : Block_Data;
      Data_Len  : Interfaces.C.size_t;
      New_Hash  : out Hash_Array;
      Status    : out Interfaces.C.int)
   is
      Next : Interfaces.C.size_t;
      Block_Data_Addr : System.Address;
   begin
      if Count >= Interfaces.C.size_t (Max_Blocks) then
         Status := -1;
         return;
      end if;

      Next := Count + 1;

      Blocks (Integer (Next)).Prev_Hash := Prev_Hash;
      Blocks (Integer (Next)).Data := Data;
      Blocks (Integer (Next)).Data_Len := Data_Len;

      Block_Data_Addr := Blocks (Integer (Next)).Data'Address;
      Blocks (Integer (Next)).Hash := SHA256 (Block_Data_Addr, Data_Len);

      New_Hash := Blocks (Integer (Next)).Hash;

      Count := Next;
      Status := 0;
   end Add_Block;

   procedure Verify_Chain
     (Start_Index : Interfaces.C.size_t;
      End_Index   : Interfaces.C.size_t;
      Status      : out Interfaces.C.int) is
   begin
      if Start_Index = 0 or else End_Index > Count or else Start_Index > End_Index then
         Status := -1;
         return;
      end if;

      for I in Integer (Start_Index) .. Integer (End_Index) - 1 loop
         if Blocks (I).Hash /= Blocks (I + 1).Prev_Hash then
            Status := -1;
            return;
         end if;
      end loop;

      Status := 0;
   end Verify_Chain;

end Core_Ledger;
