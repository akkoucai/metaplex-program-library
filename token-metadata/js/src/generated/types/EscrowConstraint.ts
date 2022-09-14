/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
import { EscrowConstraintType, escrowConstraintTypeBeet } from './EscrowConstraintType';
export type EscrowConstraint = {
  name: string;
  tokenLimit: beet.bignum;
  constraintType: EscrowConstraintType;
};

/**
 * @category userTypes
 * @category generated
 */
export const escrowConstraintBeet = new beet.FixableBeetArgsStruct<EscrowConstraint>(
  [
    ['name', beet.utf8String],
    ['tokenLimit', beet.u64],
    ['constraintType', escrowConstraintTypeBeet],
  ],
  'EscrowConstraint',
);
