with IDL_Types;
with Interfaces.C;
with System;

package SCC_IDL_Parser is
   pragma SPARK_Mode (On);

   pragma Preelaborate;

   subtype Parser_Status is Interfaces.C.int range -1 .. 1;

   PARSER_SUCCESS : constant Parser_Status := 0;
   PARSER_ERROR   : constant Parser_Status := -1;

   MAX_IDL_LENGTH : constant Interfaces.C.size_t := 65536;
   MAX_TOKEN_COUNT : constant Interfaces.C.size_t := 1024;

   type Token_Type is (TOKEN_IDENT, TOKEN_KEYWORD, TOKEN_NUMBER,
                       TOKEN_LBRACE, TOKEN_RBRACE, TOKEN_LBRACKET,
                       TOKEN_RBRACKET, TOKEN_LPAREN, TOKEN_RPAREN,
                       TOKEN_SEMICOLON, TOKEN_COMMA, TOKEN_COLON,
                       TOKEN_STRING, TOKEN_EOF, TOKEN_INVALID);
   pragma Convention (C, Token_Type);

   type Token_Record is record
      Token_Kind : Token_Type;
      Value      : Interfaces.C.char_array (1 .. 128);
      Value_Len  : Interfaces.C.size_t;
      Line       : Interfaces.C.size_t;
      Column     : Interfaces.C.size_t;
   end record;
   pragma Convention (C, Token_Record);

   type Token_Array is array (1 .. MAX_TOKEN_COUNT) of Token_Record;
   type Token_Array_Access is access all Token_Array;

   type AST_Node_Kind is (NODE_ROOT, NODE_STRUCT, NODE_FIELD, NODE_ARRAY, NODE_PRIMITIVE);
   pragma Convention (C, AST_Node_Kind);

   type AST_Node;
   type AST_Node_Access is access all AST_Node;
   type AST_Node_Array is array (1 .. MAX_TOKEN_COUNT) of AST_Node_Access;
   type AST_Node_Array_Access is access all AST_Node_Array;

   type AST_Node (Kind : AST_Node_Kind := NODE_ROOT) is record
      Name      : Interfaces.C.char_array (1 .. 64);
      Name_Len  : Interfaces.C.size_t;
      IDL_Type  : IDL_Types.IDL_Type_Access;
      Child_Count : Interfaces.C.size_t;
      Children  : AST_Node_Array_Access;
      case Kind is
         when NODE_ROOT =>
            Struct_Count : Interfaces.C.size_t;
         when NODE_STRUCT =>
            Field_Count : Interfaces.C.size_t;
         when NODE_FIELD =>
            Type_Kind : IDL_Types.IDL_Kind;
            Array_Len : Interfaces.C.size_t;
         when NODE_ARRAY | NODE_PRIMITIVE =>
            null;
      end case;
   end record;
   pragma Convention (C, AST_Node);

   type Parser_Context is record
      Content   : System.Address;
      Content_Len : Interfaces.C.size_t;
      Position  : Interfaces.C.size_t;
      Line      : Interfaces.C.size_t;
      Column    : Interfaces.C.size_t;
      Token_Ptr : Token_Array_Access;
      Token_Count : Interfaces.C.size_t;
      Current_Token : Interfaces.C.size_t;
      Root_Node : AST_Node_Access;
   end record;
   pragma Convention (C, Parser_Context);

   type Parser_Context_Access is access all Parser_Context;

   procedure Parse_String
     (Content      : Interfaces.C.char_array;
      Content_Len  : Interfaces.C.size_t;
      Root_Out     : out System.Address;
      Status       : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "scc_idl_parse_string",
          Depends => (Status => (Content, Content_Len),
                      Root_Out => (Content, Content_Len)),
          Pre => Content_Len <= MAX_IDL_LENGTH,
          Post => (if Status = PARSER_SUCCESS then Root_Out /= System.Null_Address);

   procedure Free_IDL_Tree
     (Root : System.Address)
     with Export,
          Convention => C,
          External_Name => "scc_idl_free_tree",
          Depends => (null => Root),
          Pre => Root /= System.Null_Address;

   procedure Get_Last_Error
     (Error_Msg : out Interfaces.C.char_array;
      Error_Len : out Interfaces.C.size_t)
     with Export,
          Convention => C,
          External_Name => "scc_idl_get_last_error";

private
   pragma Export (C, Parse_String, "scc_idl_parse_string");
   pragma Export (C, Free_IDL_Tree, "scc_idl_free_tree");
   pragma Export (C, Get_Last_Error, "scc_idl_get_last_error");

end SCC_IDL_Parser;
