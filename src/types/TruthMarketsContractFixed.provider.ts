/**
* This file was automatically generated by @cosmwasm/ts-codegen@1.12.1.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { ContractBase, IContractConstructor, IEmptyClient } from "./contractContextBase";
import { TruthMarketsContractFixedClient, TruthMarketsContractFixedQueryClient } from "./TruthMarketsContractFixed.client";
export class TruthMarketsContractFixed extends ContractBase<TruthMarketsContractFixedClient, TruthMarketsContractFixedQueryClient, IEmptyClient> {
  constructor({
    address,
    cosmWasmClient,
    signingCosmWasmClient
  }: IContractConstructor) {
    super(address, cosmWasmClient, signingCosmWasmClient, TruthMarketsContractFixedClient, TruthMarketsContractFixedQueryClient, undefined);
  }
}