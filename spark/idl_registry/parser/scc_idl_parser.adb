with IDL_Types;
with Interfaces.C;
with System;
with Ada.Unchecked_Deallocation;
with Ada.Strings.Fixed;

package body SCC_IDL_Parser is
   pragma SPARK_Mode (Off);
   --  Cannot prove dynamic allocation (new, Unchecked_Deallocation)
   use type Interfaces.C.int;
   use type Interfaces.C.size_t;
   use type System.Address;

   procedure Free is new Ada.Unchecked_Deallocation (AST_Node, AST_Node_Access);
   procedure Free is new Ada.Unchecked_Deallocation (AST_Node_Array, AST_Node_Array_Access);
   procedure Free is new Ada.Unchecked_Deallocation (Token_Array, Token_Array_Access);
   procedure Free is new Ada.Unchecked_Deallocation (Parser_Context, Parser_Context_Access);

   Last_Error_Msg : Interfaces.C.char_array (1 .. 256);
   Last_Error_Len : Interfaces.C.size_t := 0;

   procedure Set_Error (Msg : String) is
   begin
      Last_Error_Len := Msg'Length;
      if Last_Error_Len > 255 then
         Last_Error_Len := 255;
      end if;
      for I in 1 .. Last_Error_Len loop
         Last_Error_Msg (I) := Interfaces.C.char'Val (Character'Pos (Msg (Msg'First + I - 1)));
      end loop;
      for I in Last_Error_Len + 1 .. 256 loop
         Last_Error_Msg (I) := Interfaces.C.char'Val (0);
      end loop;
   end Set_Error;

   function Get_Char (Ctx : Parser_Context_Access; Pos : Interfaces.C.size_t) return Interfaces.C.char is
   begin
      if Pos >= Ctx.Content_Len then
         return Interfaces.C.char'Val (0);
      end if;
      declare
         Content_Str : constant Interfaces.C.char_array := Ctx.Content.all;
      begin
         return Content_Str (Content_Str'First + Interfaces.C.size_t'Pos (Pos));
      end;
   end Get_Char;

   procedure Skip_Whitespace (Ctx : Parser_Context_Access) is
      Ch : Interfaces.C.char;
   begin
      while Ctx.Position < Ctx.Content_Len loop
         Ch := Get_Char (Ctx, Ctx.Position);
         exit when Ch /= ' ' and then Ch /= ASCII.HT and then Ch /= ASCII.LF and then Ch /= ASCII.CR;
         if Ch = ASCII.LF then
            Ctx.Line := Ctx.Line + 1;
            Ctx.Column := 1;
         else
            Ctx.Column := Ctx.Column + 1;
         end if;
         Ctx.Position := Ctx.Position + 1;
      end loop;
   end Skip_Whitespace;

   procedure Next_Token (Ctx : Parser_Context_Access; Tok : out Token_Record) is
      Ch      : Interfaces.C.char;
      Start   : Interfaces.C.size_t;
   begin
      Skip_Whitespace (Ctx);
      Tok := (Token_Kind => TOKEN_INVALID, Value => (others => Interfaces.C.char'Val (0)),
              Value_Len => 0, Line => Ctx.Line, Column => Ctx.Column);

      if Ctx.Position >= Ctx.Content_Len then
         Tok.Token_Kind := TOKEN_EOF;
         return;
      end if;

      Ch := Get_Char (Ctx, Ctx.Position);

      case Interfaces.C.int'Val (Interfaces.C.char'Pos (Ch)) is
         when Character'Pos ('{') =>
            Tok.Token_Kind := TOKEN_LBRACE;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos ('}') =>
            Tok.Token_Kind := TOKEN_RBRACE;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos ('[') =>
            Tok.Token_Kind := TOKEN_LBRACKET;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos (']') =>
            Tok.Token_Kind := TOKEN_RBRACKET;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos ('(') =>
            Tok.Token_Kind := TOKEN_LPAREN;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos (')') =>
            Tok.Token_Kind := TOKEN_RPAREN;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos (';') =>
            Tok.Token_Kind := TOKEN_SEMICOLON;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos (',') =>
            Tok.Token_Kind := TOKEN_COMMA;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos (':') =>
            Tok.Token_Kind := TOKEN_COLON;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
         when Character'Pos ('"') =>
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
            Start := Ctx.Position;
            Tok.Token_Kind := TOKEN_STRING;
            while Ctx.Position < Ctx.Content_Len loop
               Ch := Get_Char (Ctx, Ctx.Position);
               exit when Ch = '"';
               Ctx.Position := Ctx.Position + 1;
            end loop;
            Tok.Value_Len := Ctx.Position - Start;
            if Tok.Value_Len > 127 then
               Tok.Value_Len := 127;
            end if;
            for I in 1 .. Tok.Value_Len loop
               Tok.Value (I) := Get_Char (Ctx, Start + I - 1);
            end loop;
            if Ctx.Position < Ctx.Content_Len then
               Ctx.Position := Ctx.Position + 1;
            end if;
         when Character'Pos ('0') .. Character'Pos ('9') =>
            Start := Ctx.Position;
            Tok.Token_Kind := TOKEN_NUMBER;
            while Ctx.Position < Ctx.Content_Len loop
               Ch := Get_Char (Ctx, Ctx.Position);
               exit when Ch < '0' or else Ch > '9';
               Ctx.Position := Ctx.Position + 1;
            end loop;
            Tok.Value_Len := Ctx.Position - Start;
            if Tok.Value_Len > 127 then
               Tok.Value_Len := 127;
            end if;
            for I in 1 .. Tok.Value_Len loop
               Tok.Value (I) := Get_Char (Ctx, Start + I - 1);
            end loop;
         when Character'Pos ('a') .. Character'Pos ('z') |
              Character'Pos ('A') .. Character'Pos ('Z') |
              Character'Pos ('_') =>
            Start := Ctx.Position;
            Tok.Token_Kind := TOKEN_IDENT;
            while Ctx.Position < Ctx.Content_Len loop
               Ch := Get_Char (Ctx, Ctx.Position);
               exit when (Ch < 'a' or else Ch > 'z') and then
                         (Ch < 'A' or else Ch > 'Z') and then
                         (Ch < '0' or else Ch > '9') and then
                         Ch /= '_';
               Ctx.Position := Ctx.Position + 1;
            end loop;
            Tok.Value_Len := Ctx.Position - Start;
            if Tok.Value_Len > 127 then
               Tok.Value_Len := 127;
            end if;
            for I in 1 .. Tok.Value_Len loop
               Tok.Value (I) := Get_Char (Ctx, Start + I - 1);
            end loop;
            if Tok.Value_Len = 4 then
               if (Tok.Value (1) = 'u' or else Tok.Value (1) = 'U') and then
                  (Tok.Value (2) = '8' or else Tok.Value (2) = '8') and then
                  Tok.Value (3) = ' ' then
                  Tok.Token_Kind := TOKEN_KEYWORD;
               end if;
            end if;
         when others =>
            Tok.Token_Kind := TOKEN_INVALID;
            Ctx.Position := Ctx.Position + 1;
            Ctx.Column := Ctx.Column + 1;
      end case;
   end Next_Token;

   procedure Expect_Token (Ctx : Parser_Context_Access; Expected : Token_Type; Tok : out Token_Record) is
   begin
      Next_Token (Ctx, Tok);
      if Tok.Token_Kind /= Expected then
         Set_Error ("Expected token " & Expected'Image & " but got " & Tok.Token_Kind'Image);
      end if;
   end Expect_Token;

   function Parse_Type (Ctx : Parser_Context_Access; Tok : Token_Record) return IDL_Types.IDL_Kind is
   begin
      if Tok.Value_Len >= 2 and then Tok.Value (1) = 'u' and then Tok.Value (2) = '8' then
         return IDL_Types.Kind_U8;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'u' and then Tok.Value (2) = '1' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '6' then
            return IDL_Types.Kind_U16;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'u' and then Tok.Value (2) = '3' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '2' then
            return IDL_Types.Kind_U32;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'u' and then Tok.Value (2) = '6' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '4' then
            return IDL_Types.Kind_U64;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'i' and then Tok.Value (2) = '8' then
         return IDL_Types.Kind_I8;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'i' and then Tok.Value (2) = '1' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '6' then
            return IDL_Types.Kind_I16;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'i' and then Tok.Value (2) = '3' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '2' then
            return IDL_Types.Kind_I32;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'i' and then Tok.Value (2) = '6' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '4' then
            return IDL_Types.Kind_I64;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'f' and then Tok.Value (2) = '3' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '2' then
            return IDL_Types.Kind_F32;
         end if;
      elsif Tok.Value_Len >= 2 and then Tok.Value (1) = 'f' and then Tok.Value (2) = '6' then
         if Tok.Value_Len >= 3 and then Tok.Value (3) = '4' then
            return IDL_Types.Kind_F64;
         end if;
      end if;
      return IDL_Types.Kind_String;
   end Parse_Type;

   function Parse_Struct (Ctx : Parser_Context_Access) return AST_Node_Access is
      Tok      : Token_Record;
      Node     : AST_Node_Access := new AST_Node (Kind => NODE_STRUCT);
      Field    : AST_Node_Access;
   begin
      Expect_Token (Ctx, TOKEN_LBRACE, Tok);

      Node.Field_Count := 0;
      loop
         Next_Token (Ctx, Tok);
         exit when Tok.Token_Kind = TOKEN_RBRACE;
         exit when Tok.Token_Kind = TOKEN_EOF;

         if Tok.Token_Kind = TOKEN_IDENT then
            Field := new AST_Node (Kind => NODE_FIELD);
            Field.Name_Len := Tok.Value_Len;
            for I in 1 .. Tok.Value_Len loop
               Field.Name (I) := Tok.Value (I);
            end loop;
            for I in Tok.Value_Len + 1 .. 64 loop
               Field.Name (I) := Interfaces.C.char'Val (0);
            end loop;

            Next_Token (Ctx, Tok);
            Field.Type_Kind := Parse_Type (Ctx, Tok);

            Next_Token (Ctx, Tok);
            exit when Tok.Token_Kind = TOKEN_SEMICOLON;

            if Tok.Token_Kind = TOKEN_LBRACKET then
               Next_Token (Ctx, Tok);
               if Tok.Token_Kind = TOKEN_NUMBER then
                  declare
                     Num_Str : String (1 .. Integer (Tok.Value_Len));
                  begin
                     for I in 1 .. Integer (Tok.Value_Len) loop
                        Num_Str (I) := Character'Val (Interfaces.C.char'Pos (Tok.Value (I)));
                     end loop;
                     Field.Array_Len := Interfaces.C.size_t'Value (Num_Str);
                  end;
               end if;
               Next_Token (Ctx, Tok);
            end if;

            Node.Field_Count := Node.Field_Count + 1;
         end if;
      end loop;

      return Node;
   exception
      when others =>
         if Node /= null then
            Free (Node);
         end if;
         raise;
   end Parse_Struct;

   procedure Parse_String
     (Content      : Interfaces.C.char_array;
      Content_Len  : Interfaces.C.size_t;
      Root_Out     : out System.Address;
      Status       : out Interfaces.C.int)
   is
      Ctx     : Parser_Context_Access := new Parser_Context;
      Tok     : Token_Record;
      Root    : AST_Node_Access;
   begin
      if Content_Len > MAX_IDL_LENGTH then
         Set_Error ("IDL content exceeds maximum length");
         Status := PARSER_ERROR;
         Root_Out := System.Null_Address;
         return;
      end if;

      if Content'Length = 0 then
         Set_Error ("Empty IDL content");
         Status := PARSER_ERROR;
         Root_Out := System.Null_Address;
         return;
      end if;

      Ctx.Content := Content'Address;
      Ctx.Content_Len := Content_Len;
      Ctx.Position := 0;
      Ctx.Line := 1;
      Ctx.Column := 1;
      Ctx.Token_Ptr := new Token_Array;
      Ctx.Token_Count := 0;
      Ctx.Current_Token := 0;

      Root := new AST_Node (Kind => NODE_ROOT);
      Root.Struct_Count := 0;
      Ctx.Root_Node := Root;

      loop
         Next_Token (Ctx, Tok);
         exit when Tok.Token_Kind = TOKEN_EOF;

         if Tok.Token_Kind = TOKEN_KEYWORD then
            Next_Token (Ctx, Tok);
            if Tok.Token_Kind = TOKEN_IDENT then
               declare
                  Struct_Node : AST_Node_Access := Parse_Struct (Ctx);
               begin
                  Struct_Node.Name_Len := Tok.Value_Len;
                  for I in 1 .. Tok.Value_Len loop
                     Struct_Node.Name (I) := Tok.Value (I);
                  end loop;
                  for I in Tok.Value_Len + 1 .. 64 loop
                     Struct_Node.Name (I) := Interfaces.C.char'Val (0);
                  end loop;
                  Root.Struct_Count := Root.Struct_Count + 1;
               end;
            end if;
         end if;
      end loop;

      Root_Out := System.Address (Root);
      Status := PARSER_SUCCESS;

   exception
      when E : others =>
         Set_Error ("Parse error: " & Ada.IO_Exceptions.Exception_Name (E));
         if Root /= null then
            Free_IDL_Tree (System.Address (Root));
         end if;
         if Ctx /= null then
            if Ctx.Token_Ptr /= null then
               Free (Ctx.Token_Ptr);
            end if;
            Free (Ctx);
         end if;
         Status := PARSER_ERROR;
         Root_Out := System.Null_Address;
   end Parse_String;

   procedure Free_IDL_Tree
     (Root : System.Address)
   is
      Node : AST_Node_Access;
   begin
      if Root = System.Null_Address then
         return;
      end if;
      Node := AST_Node_Access (Root);
      if Node.Children /= null then
         for I in 1 .. Node.Child_Count loop
            if Node.Children.all (I) /= null then
               Free_IDL_Tree (System.Address (Node.Children.all (I)));
            end if;
         end loop;
         Free (Node.Children);
      end if;
      Free (Node);
   end Free_IDL_Tree;

   procedure Get_Last_Error
     (Error_Msg : out Interfaces.C.char_array;
      Error_Len : out Interfaces.C.size_t)
   is
   begin
      Error_Len := Last_Error_Len;
      for I in 1 .. 256 loop
         Error_Msg (I) := Last_Error_Msg (I);
      end loop;
   end Get_Last_Error;

end SCC_IDL_Parser;
