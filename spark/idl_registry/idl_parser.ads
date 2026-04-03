--  IDL Parser – phân tích ngôn ngữ IDL để xây dựng cây cú pháp.
--  Các định nghĩa kiểu được xuất sang Rust để sử dụng trong translator.

with Interfaces.C;
with IDL_Types;
with System;

package IDL_Parser with SPARK_Mode => On is

   pragma Preelaborate;

   subtype Parser_Status is Interfaces.C.int range -1 .. 1;

   PARSER_SUCCESS : constant Parser_Status := 0;
   PARSER_ERROR   : constant Parser_Status := -1;

   MAX_IDL_LENGTH : constant Interfaces.C.size_t := 65536;

   procedure Parse_String
     (Content      : Interfaces.C.char_array;
      Content_Len  : Interfaces.C.size_t;
      Root_Out     : out System.Address;
      Status       : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "idl_parse_string",
          Depends => (Status => (Content, Content_Len),
                      Root_Out => (Content, Content_Len)),
          Pre => Content_Len <= MAX_IDL_LENGTH,
          Post => (if Status = PARSER_SUCCESS then Root_Out /= System.Null_Address);

   procedure Free_IDL_Tree
     (Root : System.Address)
     with Export,
          Convention => C,
          External_Name => "idl_free_tree",
          Depends => (null => Root),
          Pre => Root /= System.Null_Address;

end IDL_Parser;