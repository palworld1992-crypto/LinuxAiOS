with Interfaces.C;
with IDL_Types;
with System;

package body IDL_Parser is
   use type Interfaces.C.int;
   use type Interfaces.C.size_t;
   use type System.Address;

   type Token_Kind is (TOKEN_EOF, TOKEN_IDENT, TOKEN_KEYWORD, TOKEN_NUMBER,
                      TOKEN_LBRACE, TOKEN_RBRACE, TOKEN_LBRACKET,
                      TOKEN_RBRACKET, TOKEN_SEMICOLON, TOKEN_COMMA,
                      TOKEN_COLON, TOKEN_STRING, TOKEN_INVALID);
   pragma Convention (C, Token_Kind);

   type Token is record
      Kind : Token_Kind;
      Value : Interfaces.C.char_array (1 .. 128);
      Value_Len : Interfaces.C.size_t;
      Line : Interfaces.C.size_t;
      Column : Interfaces.C.size_t;
   end record;
   pragma Convention (C, Token);

   type Token_Array is array (1 .. 1024) of Token;
   type Token_Array_Access is access all Token_Array;

   type Parser is record
      Content : System.Address;
      Content_Len : Interfaces.C.size_t;
      Position : Interfaces.C.size_t;
      Line : Interfaces.C.size_t;
      Column : Interfaces.C.size_t;
      Tokens : Token_Array_Access;
      Token_Count : Interfaces.C.size_t;
      Current_Token : Interfaces.C.size_t;
   end record;
   pragma Convention (C, Parser);

   type Parser_Access is access all Parser;

   function Get_Char (P : Parser_Access; Pos : Interfaces.C.size_t) return Interfaces.C.char is
      Content_Arr : Interfaces.C.char_array;
      for Content_Arr'Address use P.Content;
   begin
      if Pos >= P.Content_Len then
         return Interfaces.C.char'Val (0);
      end if;
      return Content_Arr (Content_Arr'First + Interfaces.C.size_t'Pos (Pos));
   end Get_Char;

   procedure Skip_Whitespace (P : Parser_Access) is
      Ch : Interfaces.C.char;
   begin
      while P.Position < P.Content_Len loop
         Ch := Get_Char (P, P.Position);
         exit when Ch /= ' ' and then Ch /= ASCII.HT and then Ch /= ASCII.LF and then Ch /= ASCII.CR;
         if Ch = ASCII.LF then
            P.Line := P.Line + 1;
            P.Column := 1;
         else
            P.Column := P.Column + 1;
         end if;
         P.Position := P.Position + 1;
      end loop;
   end Skip_Whitespace;

   procedure Next_Token (P : Parser_Access; Tok : out Token) is
      Ch : Interfaces.C.char;
      Start : Interfaces.C.size_t;
   begin
      Skip_Whitespace (P);
      Tok := (Kind => TOKEN_INVALID, Value => (others => Interfaces.C.char'Val (0)),
              Value_Len => 0, Line => P.Line, Column => P.Column);

      if P.Position >= P.Content_Len then
         Tok.Kind := TOKEN_EOF;
         return;
      end if;

      Ch := Get_Char (P, P.Position);

      case Interfaces.C.int'Val (Interfaces.C.char'Pos (Ch)) is
         when Character'Pos ('{') =>
            Tok.Kind := TOKEN_LBRACE;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos ('}') =>
            Tok.Kind := TOKEN_RBRACE;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos ('[') =>
            Tok.Kind := TOKEN_LBRACKET;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos (']') =>
            Tok.Kind := TOKEN_RBRACKET;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos (';') =>
            Tok.Kind := TOKEN_SEMICOLON;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos (',') =>
            Tok.Kind := TOKEN_COMMA;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos (':') =>
            Tok.Kind := TOKEN_COLON;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
         when Character'Pos ('"') =>
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
            Start := P.Position;
            Tok.Kind := TOKEN_STRING;
            while P.Position < P.Content_Len loop
               Ch := Get_Char (P, P.Position);
               exit when Ch = '"';
               P.Position := P.Position + 1;
            end loop;
            Tok.Value_Len := P.Position - Start;
            if Tok.Value_Len > 127 then
               Tok.Value_Len := 127;
            end if;
            for I in 1 .. Tok.Value_Len loop
               Tok.Value (I) := Get_Char (P, Start + I - 1);
            end loop;
            if P.Position < P.Content_Len then
               P.Position := P.Position + 1;
            end if;
         when Character'Pos ('0') .. Character'Pos ('9') =>
            Start := P.Position;
            Tok.Kind := TOKEN_NUMBER;
            while P.Position < P.Content_Len loop
               Ch := Get_Char (P, P.Position);
               exit when Ch < '0' or else Ch > '9';
               P.Position := P.Position + 1;
            end loop;
            Tok.Value_Len := P.Position - Start;
            if Tok.Value_Len > 127 then
               Tok.Value_Len := 127;
            end if;
            for I in 1 .. Tok.Value_Len loop
               Tok.Value (I) := Get_Char (P, Start + I - 1);
            end loop;
         when Character'Pos ('a') .. Character'Pos ('z') |
              Character'Pos ('A') .. Character'Pos ('Z') |
              Character'Pos ('_') =>
            Start := P.Position;
            Tok.Kind := TOKEN_IDENT;
            while P.Position < P.Content_Len loop
               Ch := Get_Char (P, P.Position);
               exit when (Ch < 'a' or else Ch > 'z') and then
                         (Ch < 'A' or else Ch > 'Z') and then
                         (Ch < '0' or else Ch > '9') and then
                         Ch /= '_';
               P.Position := P.Position + 1;
            end loop;
            Tok.Value_Len := P.Position - Start;
            if Tok.Value_Len > 127 then
               Tok.Value_Len := 127;
            end if;
            for I in 1 .. Tok.Value_Len loop
               Tok.Value (I) := Get_Char (P, Start + I - 1);
            end loop;
            if Tok.Value_Len >= 2 and then Tok.Value (1) = 'u' and then Tok.Value (2) = '8' then
               Tok.Kind := TOKEN_KEYWORD;
            end if;
         when others =>
            Tok.Kind := TOKEN_INVALID;
            P.Position := P.Position + 1;
            P.Column := P.Column + 1;
      end case;
   end Next_Token;

   procedure Parse_String
     (Content      : Interfaces.C.char_array;
      Content_Len  : Interfaces.C.size_t;
      Root_Out     : out System.Address;
      Status       : out Interfaces.C.int)
   is
      P : Parser_Access;
      Tok : Token;
   begin
      if Content_Len > MAX_IDL_LENGTH then
         Status := PARSER_ERROR;
         Root_Out := System.Null_Address;
         return;
      end if;

      if Content'Length = 0 then
         Status := PARSER_ERROR;
         Root_Out := System.Null_Address;
         return;
      end if;

      P := new Parser;
      P.Content := Content'Address;
      P.Content_Len := Content_Len;
      P.Position := 0;
      P.Line := 1;
      P.Column := 1;
      P.Tokens := new Token_Array;
      P.Token_Count := 0;
      P.Current_Token := 0;

      loop
         exit when P.Position >= P.Content_Len;
         Next_Token (P, Tok);
         exit when Tok.Kind = TOKEN_EOF;
         P.Token_Count := P.Token_Count + 1;
         if P.Token_Count <= 1024 then
            P.Tokens.all (Integer (P.Token_Count)) := Tok;
         end if;
      end loop;

      Root_Out := System.Address (P.Tokens);
      Status := PARSER_SUCCESS;

   exception
      when others =>
         Status := PARSER_ERROR;
         Root_Out := System.Null_Address;
   end Parse_String;

   procedure Free_IDL_Tree
     (Root : System.Address)
   is
      Tok_Arr : Token_Array_Access;
   begin
      if Root = System.Null_Address then
         return;
      end if;
      Tok_Arr := Token_Array_Access (Root);
      if Tok_Arr /= null then
         for I in Tok_Arr.all'Range loop
            Tok_Arr.all (I).Value := (others => Interfaces.C.char'Val (0));
         end loop;
      end if;
   end Free_IDL_Tree;

end IDL_Parser;
