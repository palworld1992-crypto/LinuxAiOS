--  SPARK Main Package – exports all SPARK modules for AIOS
--  Phase 2: IDL Registry, Crypto Engine, KMS, Identity Manager, Core Ledger

with Interfaces.C;
with System;

package Spark is
   pragma SPARK_Mode (On);
   pragma Pure;

   -- Version information
   SPARK_Version : constant String := "0.2.0";

   ------------------------------------------------------------------------
   -- IDL Registry Components
   ------------------------------------------------------------------------
   --  IDL_Types: Type definitions for IDL parsing
   --  Type_Mapper: Maps IDL types to C layout
   --  Layout_Calculator: Computes alignment and offsets
   --  IDL_Parser: Parses IDL strings into AST

   ------------------------------------------------------------------------
   -- Crypto Engine Components
   ------------------------------------------------------------------------
   --  Crypto_Engine: AES-GCM, HMAC-SHA256, Kyber, Dilithium
   --  All crypto operations use SPARK_Mode (Off) in bodies due to FFI

   ------------------------------------------------------------------------
   -- KMS and Identity
   ------------------------------------------------------------------------
   --  KMS: Key management with formal verification
   --  Identity_Manager: Token creation and verification

   ------------------------------------------------------------------------
   -- Core Ledger
   ------------------------------------------------------------------------
   --  Core_Ledger: Blockchain for health records with SPARK verification

   -- Status codes for SPARK operations
   type Spark_Status is (SPARK_OK, SPARK_ERROR, SPARK_INVALID_INPUT);
   pragma Convention (C, Spark_Status);

   -- Version query procedure (C API)
   procedure Get_Version
     (Version_Out : out Interfaces.C.char_array;
      Version_Len : out Interfaces.C.size_t)
      with Export,
           Convention => C,
           External_Name => "spark_get_version",
           Depends => (Version_Out => Version_Out,
                       Version_Len => null);

end Spark;
