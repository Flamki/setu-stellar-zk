#![cfg(test)]

use ark_bls12_381::{Fq, Fq2};
use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use soroban_sdk::{
    Bytes, Env, U256, Vec,
    crypto::bls12_381::{Fr, G1_SERIALIZED_SIZE, G1Affine, G2_SERIALIZED_SIZE, G2Affine},
};
use std::{format, string::String, string::ToString};

use crate::{Groth16Error, Groth16Verifier, Proof, PublicSignals, VerificationKey};

fn g1_from_coords(env: &Env, x: &str, y: &str) -> G1Affine {
    let ark_g1 = ark_bls12_381::G1Affine::new(Fq::from_str(x).unwrap(), Fq::from_str(y).unwrap());
    let mut buf = [0u8; G1_SERIALIZED_SIZE];
    ark_g1.serialize_uncompressed(&mut buf[..]).unwrap();
    G1Affine::from_array(env, &buf)
}

fn g2_from_coords(env: &Env, x1: &str, x2: &str, y1: &str, y2: &str) -> G2Affine {
    let x = Fq2::new(Fq::from_str(x1).unwrap(), Fq::from_str(x2).unwrap());
    let y = Fq2::new(Fq::from_str(y1).unwrap(), Fq::from_str(y2).unwrap());
    let ark_g2 = ark_bls12_381::G2Affine::new(x, y);
    let mut buf = [0u8; G2_SERIALIZED_SIZE];
    ark_g2.serialize_uncompressed(&mut buf[..]).unwrap();
    G2Affine::from_array(env, &buf)
}

#[test]
fn test_with_hardcoded_vk() {
    // Initialize the test environment
    let env = Env::default();

    // Load verification key components (copied from `data/verification_key.json`)
    // These values are pre-computed for the circuit that verifies a*b = c
    // where a=3, b=11, c=33 and only c is public.
    let alphax = "851850525556173310373115880154698084608631105506432893865500290442025919078535925294035153152030470398262539759609";
    let alphay = "2637289349983507610125993281171282870664683328789064436670091381805667870657250691837988574635646688089951719927247";

    let betax1 = "1312620381151154625549413690218290437739613987001512553647554932245743783919690104921577716179019375920325686841943";
    let betax2 = "1853421227732662200477195678252233549930451033531229987959164216695698667330234953033341200627605777603511819497457";
    let betay1 = "3215807833988244618006117550809420301978856703407297742347804415291049013404133666905173282837707341742014140541018";
    let betay2 = "812366606879346135498483310623227330050424196838294715759414425317592599094348477520229174120664109186562798527696";

    let gammax1 = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
    let gammax2 = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
    let gammay1 = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
    let gammay2 = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";

    let deltax1 = "2981843938988033214458466658185878126396080429969635248100956025957789319926032198626745120548947333202362392267114";
    let deltax2 = "2236695112259305382987038341098587500598216646308901956168137697892380899086228863246537938263638056666003066263342";
    let deltay1 = "717163810166643254871951856655865822196000925757284470845197358532703820821048809982340614428800986999944933231635";
    let deltay2 = "3496058064578305387608803828034117220735807855182872031001942587835768203820179263722136810383631418598310938506798";

    let ic0x = "829685638389803071404995253486571779300247099942205634643821309129201420207693030476756893332812706176564514055395";
    let ic0y = "3455508165409829148751617737772894557887792278044850553785496869183933597103951941805834639972489587640583544390358";

    let ic1x = "2645559270376031734407122278942646687260452979296081924477586893972449945444985371392950465676350735694002713633589";
    let ic1y = "2241039659097418315097403108596818813895651201896886552939297756980670248638746432560267634304593609165964274111037";

    // Construct the verification key from the pre-computed components
    let vk = VerificationKey {
        alpha: g1_from_coords(&env, alphax, alphay),
        beta: g2_from_coords(&env, betax1, betax2, betay1, betay2),
        gamma: g2_from_coords(&env, gammax1, gammax2, gammay1, gammay2),
        delta: g2_from_coords(&env, deltax1, deltax2, deltay1, deltay2),
        ic: Vec::from_array(
            &env,
            [
                g1_from_coords(&env, ic0x, ic0y),
                g1_from_coords(&env, ic1x, ic1y),
            ],
        ),
    };

    // Load proof components (copied from `data/proof.json`)
    let pi_ax = "314442236668110257304682488877371582255161413673331360366570443799415414639292047869143313601702131653514009114222";
    let pi_ay = "2384632327855835824635705027009217874826122107057894594162233214798350178691568018290025994699762298534539543934607";
    let pi_bx1 = "428844167033934720609657613212495751617651348480870890908850335525890280786532876634895457032623422366474694342656";
    let pi_bx2 = "3083139526360252775789959298805261067575555607578161553873977966165446991459924053189383038704105379290158793353905";
    let pi_by1 = "1590919422794657666432683000821892403620510405626533455397042191265963587891653562867091397248216891852168698286910";
    let pi_by2 = "3617931039814164588401589536353142503544155307022467123698224064329647390280346725086550997337076315487486714327146";
    let pi_cx = "3052934797502613468327963344215392478880720823583493172692775426011388142569325036386650708808320216973179639719187";
    let pi_cy = "2028185281516938724429867827057869371578022471499780916652824405212207527699373814371051328341613972789943854539597";

    // Construct the proof from the pre-computed components
    let proof = Proof {
        a: g1_from_coords(&env, &pi_ax, &pi_ay),
        b: g2_from_coords(&env, &pi_bx1, &pi_bx2, &pi_by1, &pi_by2),
        c: g1_from_coords(&env, &pi_cx, &pi_cy),
    };

    // Test Case 1: Verify the proof with the correct public output (33, copied from `data/public.json`)
    let output = Vec::from_array(&env, [Fr::from_u256(U256::from_u32(&env, 33))]);
    let res = Groth16Verifier::verify_proof(&env, vk.clone(), proof.clone(), &output).unwrap();
    assert_eq!(res, true);

    // Print out the budget report showing CPU and memory cost breakdown for
    // different operations (zero-value operations omitted for brevity)
    env.cost_estimate().budget().print();
    /*
    =================================================================
    Cpu limit: 100000000; used: 40968821
    Mem limit: 41943040; used: 297494
    =================================================================
    CostType                           cpu_insns      mem_bytes
    MemAlloc                           12089          3401
    MemCpy                             3091           0
    MemCmp                             928            0
    VisitObject                        5917           0
    ComputeSha256Hash                  3738           0
    Bls12381EncodeFp                   2644           0
    Bls12381DecodeFp                   29550          0
    Bls12381G1CheckPointOnCurve        13538          0
    Bls12381G1CheckPointInSubgroup     3652550        0
    Bls12381G2CheckPointOnCurve        23684          0
    Bls12381G2CheckPointInSubgroup     4231288        0
    Bls12381G1ProjectiveToAffine       185284         0
    Bls12381G1Add                      7689           0
    Bls12381G1Mul                      2458985        0
    Bls12381Pairing                    30335852       294093
    Bls12381FrFromU256                 1994           0
    // ... zero-value rows omitted ...
    =================================================================
    */

    // Test Case 2: Verify the proof with an incorrect public output (22)
    let output = Vec::from_array(&env, [Fr::from_u256(U256::from_u32(&env, 22))]);
    let res = Groth16Verifier::verify_proof(&env, vk, proof, &output).unwrap();
    assert_eq!(res, false);
}

#[test]
fn test_with_circom2soroban_output() {
    let env = Env::default();

    let alphax = "3747424941603402268361699745001854633014744937273750680241086531929563811053644994104663362172135397158933644732769";
    let alphay = "3988270719613075850599363358389871346508135461410965432812272991775727534181494682456780460875277721404051889718365";
    let betax1 = "1524697016042884165313306760188648321951005417956121512876963277584907694454409573781160610530537485279864685673476";
    let betax2 = "412991159702677602477943588110116178996282791294527953176916401488419471086841528515891441472334815486249056797338";
    let betay1 = "3916121343675848239958862389733312831623003692196409224010750692694641149859846091758947486087378597010991240658177";
    let betay2 = "3414240119772657921827165873165712163213689442255818817262020650338672456890199595260427614694802487144556998749052";
    let gammax1 = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
    let gammax2 = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
    let gammay1 = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
    let gammay2 = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";
    let deltax1 = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
    let deltax2 = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
    let deltay1 = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
    let deltay2 = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";
    let ic0x = "474952218074335863578664511024945165344669103923167332389103736276210593389972064509878520740161937977997054054001";
    let ic0y = "603113250469777993716180248063826731461277231763492855005256665015412916162238075305366716623156236156854946844790";
    let ic1x = "2856604914760717213286948932893423171459919318753357293811398930302237923886719800243791210923391190741419079860011";
    let ic1y = "3860559014590268839847636251095308099241297494019641587854667966109705297682849364509800320911445835318007004527450";

    // Construct the verification key from the pre-computed components
    let vk = VerificationKey {
        alpha: g1_from_coords(&env, alphax, alphay),
        beta: g2_from_coords(&env, betax1, betax2, betay1, betay2),
        gamma: g2_from_coords(&env, gammax1, gammax2, gammay1, gammay2),
        delta: g2_from_coords(&env, deltax1, deltax2, deltay1, deltay2),
        ic: Vec::from_array(
            &env,
            [
                g1_from_coords(&env, ic0x, ic0y),
                g1_from_coords(&env, ic1x, ic1y),
            ],
        ),
    };

    // Load proof components (copied from `data/proof.json`)
    let pi_ax = "314442236668110257304682488877371582255161413673331360366570443799415414639292047869143313601702131653514009114222";
    let pi_ay = "2384632327855835824635705027009217874826122107057894594162233214798350178691568018290025994699762298534539543934607";
    let pi_bx1 = "428844167033934720609657613212495751617651348480870890908850335525890280786532876634895457032623422366474694342656";
    let pi_bx2 = "3083139526360252775789959298805261067575555607578161553873977966165446991459924053189383038704105379290158793353905";
    let pi_by1 = "1590919422794657666432683000821892403620510405626533455397042191265963587891653562867091397248216891852168698286910";
    let pi_by2 = "3617931039814164588401589536353142503544155307022467123698224064329647390280346725086550997337076315487486714327146";
    let pi_cx = "3052934797502613468327963344215392478880720823583493172692775426011388142569325036386650708808320216973179639719187";
    let pi_cy = "2028185281516938724429867827057869371578022471499780916652824405212207527699373814371051328341613972789943854539597";

    // Construct the proof from the pre-computed components
    let proof = Proof {
        a: g1_from_coords(&env, &pi_ax, &pi_ay),
        b: g2_from_coords(&env, &pi_bx1, &pi_bx2, &pi_by1, &pi_by2),
        c: g1_from_coords(&env, &pi_cx, &pi_cy),
    };

    let res = Groth16Verifier::verify_proof(
        &env,
        vk,
        proof,
        &Vec::from_array(&env, [Fr::from_u256(U256::from_u32(&env, 0))]),
    )
    .unwrap();
    assert_eq!(res, false); // Should be false for dummy proof/key
}

#[test]
fn test_coin_ownership() {
    let env = Env::default();

    let alphax = "2625583050305146829700663917277485398332586266229739236073977691599912239208704058548731458555934906273399977862822";
    let alphay = "1155364156944807367912876641032696519500054551629402873339575774959620483194368919563799050765095981406853619398751";

    let betax1 = "1659696755509039809248937927616726274238080235224171061036366585278216098417245587200210264410333778948851576160490";
    let betax2 = "1338363397031837211155983756179787835339490797745307535810204658838394402900152502268197396587061400659003281046656";
    let betay1 = "1974652615426136516341494326987376616840373177388374023461177997087381634383568759591087499459321812809521924259354";
    let betay2 = "3301884318087924474550898163462840036865878131635519297186391370517333773367262804074867347346141727012544462046142";

    let gammax1 = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
    let gammax2 = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
    let gammay1 = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
    let gammay2 = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";

    let deltax1 = "2743142984898738125001654242270255897674294587748253980774444623218527122281885484824857579589713606363239065733017";
    let deltax2 = "204094511116361675952446773023082620129915086661714657422228091029815576847171516978148871266279683009739984647370";
    let deltay1 = "1939174617523090044587198902486590913714778055316589513843553175656942451344239147573490910682365706772067255113505";
    let deltay2 = "2292766382025993571077921250915535065222739994205317155933298500829977470451722231881154853465745409220984289503104";

    let ic0x = "2683618448904306335228903505299721458337998659387683182703607938866093851528642837054373953710816742402346993120797";
    let ic0y = "1786668422239574992109894972831696712754414376375650177546698505846675773736122594055286256880476355557498422341634";

    let ic1x = "66902808652025389632864246790882182391974469705330059940923330385191291897071317426471178445902460845068967727278";
    let ic1y = "2594765160054401352409621725597484660493244453849646390863879503499356393964853319062162092260143573762198575717487";

    let ic2x = "1003382246631454829876446584401033316871748572555275360455886052458082997944181369047339077877767811660479191253501";
    let ic2y = "2544403050708001388892167906290198493026544815081144389732529981844829005748116399510257129962201796703233322017126";

    let vk = VerificationKey {
        alpha: g1_from_coords(&env, alphax, alphay),
        beta: g2_from_coords(&env, betax1, betax2, betay1, betay2),
        gamma: g2_from_coords(&env, gammax1, gammax2, gammay1, gammay2),
        delta: g2_from_coords(&env, deltax1, deltax2, deltay1, deltay2),
        ic: Vec::from_array(
            &env,
            [
                g1_from_coords(&env, ic0x, ic0y),
                g1_from_coords(&env, ic1x, ic1y),
                g1_from_coords(&env, ic2x, ic2y),
            ],
        ),
    };

    let pi_ax = "2312845116701672402180486748482758387792019392638873512193039748796932219258491169543785273216043839153656695561028";
    let pi_ay = "2401388344361274103911290492305041495151045197799709253975549979749000050189613887123319051631926316639875318530706";

    let pi_bx1 = "254112989406552222064547883149713450818858945843832143975529650914462737635290229325433889900886989167652287651477";
    let pi_bx2 = "1298427328165001466050889647718980726801340148316849424955517639658980366356339914458078849522724674106484718321177";
    let pi_by1 = "5321429806065285653141424032098896697927422746457669045090054322756670504823633361000243836633074177798333017419";
    let pi_by2 = "2189884843665900970576104345488554859679199450652275441615353017043226667793893188654509510191387551155638147549016";

    let pi_cx = "2191481390831460536193287377108883221181604287496545032823796584973842680176886739387559733959993660004385853733594";
    let pi_cy = "423577060414004702507038675020769607957163072861493245791725852305440453270407094427666302510671441390869114232609";

    let proof = Proof {
        a: g1_from_coords(&env, &pi_ax, &pi_ay),
        b: g2_from_coords(&env, &pi_bx1, &pi_bx2, &pi_by1, &pi_by2),
        c: g1_from_coords(&env, &pi_cx, &pi_cy),
    };

    // Public output signals:
    let public_0 = U256::from_be_bytes(
        &env,
        &Bytes::from_array(
            &env,
            &[
                0x07, 0xf5, 0x2b, 0x5b, 0x26, 0x8b, 0xf6, 0x29, 0xbc, 0x53, 0x78, 0x3e, 0xfa, 0xf4,
                0xa8, 0x20, 0x36, 0xaa, 0x76, 0xf2, 0xa5, 0x57, 0x04, 0x4f, 0xfe, 0xf6, 0x7e, 0x92,
                0xf2, 0x60, 0xf6, 0xf8,
            ],
        ),
    );
    let public_1 = U256::from_be_bytes(
        &env,
        &Bytes::from_array(
            &env,
            &[
                0x00, 0x09, 0x7b, 0xa5, 0x84, 0xee, 0xbc, 0x88, 0x32, 0xb6, 0x7d, 0x9d, 0x89, 0xd9,
                0xd2, 0xd7, 0x12, 0xfb, 0x44, 0xe0, 0x53, 0xa3, 0x0e, 0x9d, 0x1a, 0x61, 0x64, 0xcd,
                0x6b, 0x33, 0xc5, 0xde,
            ],
        ),
    );

    // Create output vector for verification:
    let output = Vec::from_array(&env, [Fr::from_u256(public_0), Fr::from_u256(public_1)]);

    let res = Groth16Verifier::verify_proof(&env, vk, proof, &output).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_vk_serde() {
    let env = Env::default();

    let alphax = "3747424941603402268361699745001854633014744937273750680241086531929563811053644994104663362172135397158933644732769";
    let alphay = "3988270719613075850599363358389871346508135461410965432812272991775727534181494682456780460875277721404051889718365";
    let betax1 = "1524697016042884165313306760188648321951005417956121512876963277584907694454409573781160610530537485279864685673476";
    let betax2 = "412991159702677602477943588110116178996282791294527953176916401488419471086841528515891441472334815486249056797338";
    let betay1 = "3916121343675848239958862389733312831623003692196409224010750692694641149859846091758947486087378597010991240658177";
    let betay2 = "3414240119772657921827165873165712163213689442255818817262020650338672456890199595260427614694802487144556998749052";
    let gammax1 = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
    let gammax2 = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
    let gammay1 = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
    let gammay2 = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";
    let deltax1 = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
    let deltax2 = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
    let deltay1 = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
    let deltay2 = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";
    let ic0x = "474952218074335863578664511024945165344669103923167332389103736276210593389972064509878520740161937977997054054001";
    let ic0y = "603113250469777993716180248063826731461277231763492855005256665015412916162238075305366716623156236156854946844790";
    let ic1x = "2856604914760717213286948932893423171459919318753357293811398930302237923886719800243791210923391190741419079860011";
    let ic1y = "3860559014590268839847636251095308099241297494019641587854667966109705297682849364509800320911445835318007004527450";

    // Construct the verification key from the pre-computed components
    let vk = VerificationKey {
        alpha: g1_from_coords(&env, alphax, alphay),
        beta: g2_from_coords(&env, betax1, betax2, betay1, betay2),
        gamma: g2_from_coords(&env, gammax1, gammax2, gammay1, gammay2),
        delta: g2_from_coords(&env, deltax1, deltax2, deltay1, deltay2),
        ic: Vec::from_array(
            &env,
            [
                g1_from_coords(&env, ic0x, ic0y),
                g1_from_coords(&env, ic1x, ic1y),
            ],
        ),
    };

    let vk_bytes = vk.to_bytes(&env);
    let deserialized_vk = VerificationKey::from_bytes(&env, &vk_bytes).unwrap();

    assert_eq!(vk.alpha, deserialized_vk.alpha);
    assert_eq!(vk.beta, deserialized_vk.beta);
    assert_eq!(vk.gamma, deserialized_vk.gamma);
    assert_eq!(vk.delta, deserialized_vk.delta);
    assert_eq!(vk.ic, deserialized_vk.ic);
}

#[test]
fn test_proof_serde() {
    let env = Env::default();

    let pi_ax = "2312845116701672402180486748482758387792019392638873512193039748796932219258491169543785273216043839153656695561028";
    let pi_ay = "2401388344361274103911290492305041495151045197799709253975549979749000050189613887123319051631926316639875318530706";

    let pi_bx1 = "254112989406552222064547883149713450818858945843832143975529650914462737635290229325433889900886989167652287651477";
    let pi_bx2 = "1298427328165001466050889647718980726801340148316849424955517639658980366356339914458078849522724674106484718321177";
    let pi_by1 = "5321429806065285653141424032098896697927422746457669045090054322756670504823633361000243836633074177798333017419";
    let pi_by2 = "2189884843665900970576104345488554859679199450652275441615353017043226667793893188654509510191387551155638147549016";

    let pi_cx = "2191481390831460536193287377108883221181604287496545032823796584973842680176886739387559733959993660004385853733594";
    let pi_cy = "423577060414004702507038675020769607957163072861493245791725852305440453270407094427666302510671441390869114232609";

    let proof = Proof {
        a: g1_from_coords(&env, &pi_ax, &pi_ay),
        b: g2_from_coords(&env, &pi_bx1, &pi_bx2, &pi_by1, &pi_by2),
        c: g1_from_coords(&env, &pi_cx, &pi_cy),
    };

    let proof_bytes = proof.to_bytes(&env);
    let deserialized_proof = Proof::from_bytes(&env, &proof_bytes).unwrap();

    assert_eq!(proof.a, deserialized_proof.a);
    assert_eq!(proof.b, deserialized_proof.b);
    assert_eq!(proof.c, deserialized_proof.c);
}

#[test]
fn test_public_signals_serde() {
    let env = Env::default();

    let pub_signals = PublicSignals {
        pub_signals: Vec::from_array(&env, [Fr::from_u256(U256::from_u32(&env, 33))]),
    };

    let pub_signals_bytes = pub_signals.to_bytes(&env);
    let deserialized_pub_signals = PublicSignals::from_bytes(&env, &pub_signals_bytes).unwrap();

    assert_eq!(
        pub_signals.pub_signals,
        deserialized_pub_signals.pub_signals
    );
}

#[test]
fn test_proof_serde_rejects_truncated_bytes() {
    let env = Env::default();
    let proof_bytes = Bytes::from_array(&env, &[0u8; 1]);

    assert_eq!(
        Proof::from_bytes(&env, &proof_bytes).err(),
        Some(Groth16Error::MalformedProof)
    );
}

#[test]
fn test_public_signals_reject_non_canonical_scalar() {
    let env = Env::default();
    let mut encoded = Bytes::from_array(&env, &[0u8, 0u8, 0u8, 1u8]);
    encoded.append(&Bytes::from_array(
        &env,
        &[
            0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1,
            0xd8, 0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x01,
        ],
    ));

    assert_eq!(
        PublicSignals::from_bytes(&env, &encoded).err(),
        Some(Groth16Error::NonCanonicalPublicSignal)
    );
}

#[test]
fn test_public_signals_reject_truncated_payload() {
    let env = Env::default();
    let encoded = Bytes::from_array(&env, &[0u8, 0u8, 0u8, 1u8, 7u8]);

    assert_eq!(
        PublicSignals::from_bytes(&env, &encoded).err(),
        Some(Groth16Error::MalformedPublicSignals)
    );
}

/// Spec: docs/public-signal-serialization.md — Withdrawal public order
/// snarkjs order: [nullifierHash, withdrawnValue, stateRoot, associationRoot].
/// Wire format: u32 BE length prefix `n`, then `n` × 32-byte big-endian Fr.
#[test]
fn test_withdrawal_public_signal_order_encoding() {
    let env = Env::default();

    let nullifier_hash = Fr::from_u256(U256::from_u32(&env, 7));
    let withdrawn_value = Fr::from_u256(U256::from_u32(&env, 1_000_000_000));
    let state_root = Fr::from_u256(U256::from_u32(&env, 2));
    let association_root = Fr::from_u256(U256::from_u32(&env, 3));

    // Index 0..3 MUST match circuits/main.circom (output then public inputs).
    let signals = PublicSignals {
        pub_signals: Vec::from_array(
            &env,
            [
                nullifier_hash,
                withdrawn_value,
                state_root,
                association_root,
            ],
        ),
    };

    let encoded = signals.to_bytes(&env);
    assert_eq!(encoded.len(), 4 + 4 * 32);

    let mut hdr = [0u8; 4];
    encoded.slice(0..4).copy_into_slice(&mut hdr);
    assert_eq!(
        hdr,
        [0u8, 0u8, 0u8, 4u8],
        "length prefix must be u32 BE n=4"
    );

    // withdrawnValue at index 1 → byte offset 4+32.
    let mut wv = [0u8; 32];
    encoded.slice((4 + 32)..(4 + 64)).copy_into_slice(&mut wv);
    assert_eq!(
        &wv[28..32],
        &[0x3b, 0x9a, 0xca, 0x00],
        "withdrawnValue 1000000000 must be big-endian at signal index 1"
    );
    assert!(wv[..28].iter().all(|b| *b == 0));

    // nullifierHash at index 0 → low byte 7 at offset 4+31.
    let mut nh = [0u8; 32];
    encoded.slice(4..(4 + 32)).copy_into_slice(&mut nh);
    assert_eq!(nh[31], 7);

    let roundtrip = PublicSignals::from_bytes(&env, &encoded).unwrap();
    assert_eq!(signals.pub_signals, roundtrip.pub_signals);
    assert_eq!(roundtrip.pub_signals.len(), 4);
}

/// Spec: docs/public-signal-serialization.md — Disclosure public order
/// [nullifierHash, commitment, discloseHash, auditorTag, receiptNonce].
#[test]
fn test_disclosure_public_signal_order_encoding() {
    let env = Env::default();

    let nullifier_hash = Fr::from_u256(U256::from_u32(&env, 11));
    let commitment = Fr::from_u256(U256::from_u32(&env, 22));
    let disclose_hash = Fr::from_u256(U256::from_u32(&env, 33));
    let auditor_tag = Fr::from_u256(U256::from_u32(&env, 44));
    let receipt_nonce = Fr::from_u256(U256::from_u32(&env, 55));

    let signals = PublicSignals {
        pub_signals: Vec::from_array(
            &env,
            [
                nullifier_hash,
                commitment,
                disclose_hash,
                auditor_tag,
                receipt_nonce,
            ],
        ),
    };

    let encoded = signals.to_bytes(&env);
    assert_eq!(encoded.len(), 4 + 5 * 32);

    let mut hdr = [0u8; 4];
    encoded.slice(0..4).copy_into_slice(&mut hdr);
    assert_eq!(hdr, [0, 0, 0, 5]);

    for (idx, expected_low) in [
        (0u32, 11u8),
        (1, 22),
        (2, 33),
        (3, 44),
        (4, 55), // receiptNonce appended after the v1 four-signal order
    ] {
        let start = 4 + idx * 32;
        let mut lane = [0u8; 32];
        encoded
            .slice(start..(start + 32))
            .copy_into_slice(&mut lane);
        assert!(lane[..31].iter().all(|b| *b == 0));
        assert_eq!(
            lane[31], expected_low,
            "disclosure signal index ordering mismatch"
        );
    }

    // Exact-length rule: trailing garbage → MalformedPublicSignals.
    let mut bloated = encoded.clone();
    bloated.append(&Bytes::from_array(&env, &[0xffu8]));
    assert_eq!(
        PublicSignals::from_bytes(&env, &bloated).err(),
        Some(Groth16Error::MalformedPublicSignals)
    );

    let roundtrip = PublicSignals::from_bytes(&env, &encoded).unwrap();
    assert_eq!(signals.pub_signals, roundtrip.pub_signals);
}

// ---------------------------------------------------------------------------
// Generated public-signal ordering tests (issue #21).
//
// Fixtures live in libs/zk/testdata/public_signals/ and carry the snarkjs
// `public.json` value order for each circuit. These tests regenerate
// `PublicSignals` from those fixtures and pin the exact index order and wire
// length against three sources that must all agree:
//   1. the order constants below (the protocol pin),
//   2. the Circom source (outputs first, then `component main {public [...]}`),
//   3. docs/public-signal-serialization.md and README.md.
// Reordering a signal in any of them fails the matching assertion.
//
// Prototype limit: the fixture values are captured from the local/staging
// single-contributor trusted setup (see each fixture's "provenance" field).
// These tests pin ordering, not proof soundness or ceremony quality.
// ---------------------------------------------------------------------------

const WITHDRAWAL_SIGNAL_ORDER: [&str; 4] = [
    "nullifierHash",
    "withdrawnValue",
    "stateRoot",
    "associationRoot",
];

const DISCLOSURE_SIGNAL_ORDER: [&str; 5] = [
    "nullifierHash",
    "commitment",
    "discloseHash",
    "auditorTag",
    "receiptNonce",
];

const WITHDRAWAL_CIRCUIT_REL: &str = "circuits/main.circom";
const DISCLOSURE_CIRCUIT_REL: &str = "circuits/disclosure.circom";
const SPEC_REL: &str = "docs/public-signal-serialization.md";

const WITHDRAWAL_FIXTURE_JSON: &str = include_str!("../testdata/public_signals/withdrawal.json");
const DISCLOSURE_FIXTURE_JSON: &str = include_str!("../testdata/public_signals/disclosure.json");
const RECEIPT_TAGS_FIXTURE_JSON: &str = include_str!("../testdata/receipt_tags.json");
const WITHDRAWAL_CIRCUIT_SRC: &str = include_str!("../../../circuits/main.circom");
const DISCLOSURE_CIRCUIT_SRC: &str = include_str!("../../../circuits/disclosure.circom");
const SPEC_DOC: &str = include_str!("../../../docs/public-signal-serialization.md");
const README_DOC: &str = include_str!("../../../README.md");

struct PublicSignalFixture {
    circuit: String,
    spec: String,
    order: std::vec::Vec<String>,
    public_signals: std::vec::Vec<String>,
}

fn json_text_field(json: &serde_json::Value, name: &str) -> String {
    match json.get(name).and_then(|value| value.as_str()) {
        Some(text) => text.to_string(),
        None => panic!("fixture field `{name}` must be a string"),
    }
}

fn json_string_array_field(json: &serde_json::Value, name: &str) -> std::vec::Vec<String> {
    let entries = match json.get(name).and_then(|value| value.as_array()) {
        Some(entries) => entries,
        None => panic!("fixture field `{name}` must be an array of strings"),
    };
    entries
        .iter()
        .map(|entry| match entry.as_str() {
            Some(text) => text.to_string(),
            None => panic!("`{name}` entries must be strings"),
        })
        .collect()
}

fn parse_public_signal_fixture(raw: &str) -> PublicSignalFixture {
    let json: serde_json::Value = serde_json::from_str(raw).expect("fixture is valid JSON");
    PublicSignalFixture {
        circuit: json_text_field(&json, "circuit"),
        spec: json_text_field(&json, "spec"),
        order: json_string_array_field(&json, "order"),
        public_signals: json_string_array_field(&json, "public_signals"),
    }
}

/// snarkjs / Circom emit public signals as circuit outputs first, then public
/// inputs in `component main {public [...]}` declaration order. Rebuild that
/// order from the circuit source so an edit to the `.circom` file is caught.
fn circuit_public_signal_order(circuit_src: &str) -> std::vec::Vec<String> {
    let mut order = std::vec::Vec::new();

    for line in circuit_src.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("signal output ") {
            let name = rest
                .split(|c: char| c.is_whitespace() || c == ';' || c == '[')
                .next()
                .unwrap_or("");
            if !name.is_empty() {
                order.push(name.to_string());
            }
        }
    }

    let marker = "{public [";
    let start = circuit_src
        .find(marker)
        .unwrap_or_else(|| panic!("circuit must declare `component main {{public [...]}}`"));
    let rest = &circuit_src[start + marker.len()..];
    let end = rest
        .find(']')
        .unwrap_or_else(|| panic!("public input list must be closed"));
    for name in rest[..end].split(',') {
        order.push(name.trim().to_string());
    }

    order
}

/// Decimal snarkjs value -> 32-byte big-endian Fr encoding (overflow checked).
fn decimal_to_be_bytes(dec: &str) -> [u8; 32] {
    assert!(!dec.is_empty(), "public signal must not be empty");
    let mut out = [0u8; 32];
    for byte in dec.bytes() {
        assert!(
            byte.is_ascii_digit(),
            "public signal `{dec}` must be a decimal integer"
        );
        let mut carry = u32::from(byte - b'0');
        for slot in out.iter_mut().rev() {
            let value = u32::from(*slot) * 10 + carry;
            *slot = value as u8;
            carry = value >> 8;
        }
        assert_eq!(carry, 0, "public signal `{dec}` does not fit in 32 bytes");
    }
    out
}

fn fr_from_decimal(env: &Env, dec: &str) -> Fr {
    Fr::from_u256(U256::from_be_bytes(
        env,
        &Bytes::from_array(env, &decimal_to_be_bytes(dec)),
    ))
}

/// The fixture, the circuit source, the spec table and the README must all
/// state the same order, and the fixture must carry one value per signal.
fn assert_fixture_pins_order(
    fixture: &PublicSignalFixture,
    pinned_order: &[&str],
    circuit_src: &str,
    circuit_rel: &str,
) {
    let fixture_order: std::vec::Vec<&str> = fixture.order.iter().map(String::as_str).collect();
    assert_eq!(
        fixture_order, pinned_order,
        "fixture signal order for {circuit_rel} changed; this pin exists to catch \
         accidental reordering (see {SPEC_REL})"
    );

    assert_eq!(
        fixture.circuit, circuit_rel,
        "fixture must point at its Circom source"
    );
    assert_eq!(
        fixture.spec, SPEC_REL,
        "fixture must point at the ordering spec it documents"
    );

    let circuit_order = circuit_public_signal_order(circuit_src);
    let circuit_order: std::vec::Vec<&str> = circuit_order.iter().map(String::as_str).collect();
    assert_eq!(
        circuit_order, pinned_order,
        "{circuit_rel} public signal order changed (outputs first, then main {{public [...]}})"
    );

    for (index, name) in pinned_order.iter().enumerate() {
        let row = format!("| {index} | `{name}` |");
        assert!(
            SPEC_DOC.contains(&row),
            "{SPEC_REL} must document `{name}` at index {index} as the table row `{row}`"
        );
    }
    assert!(
        SPEC_DOC.contains(circuit_rel),
        "{SPEC_REL} must reference {circuit_rel}"
    );

    let readme_order = format!("[{}]", pinned_order.join(", "));
    assert!(
        README_DOC.contains(&readme_order),
        "README.md must document the {circuit_rel} signal order as `{readme_order}`"
    );

    assert_eq!(
        fixture.public_signals.len(),
        pinned_order.len(),
        "fixture must carry exactly one value per public signal for {circuit_rel}"
    );

    assert_docs_quote_values_in_order(README_DOC, "README.md", fixture);
    assert_docs_quote_values_in_order(SPEC_DOC, SPEC_REL, fixture);
}

/// The README and the spec quote the fixture values as snarkjs-style decimal
/// arrays. Each value must still appear in fixture order, so swapping the
/// value at an index fails here even when the signal names are untouched.
fn assert_docs_quote_values_in_order(doc: &str, doc_rel: &str, fixture: &PublicSignalFixture) {
    let mut cursor = 0usize;
    for (index, value) in fixture.public_signals.iter().enumerate() {
        match doc[cursor..].find(value.as_str()) {
            Some(offset) => cursor += offset + value.len(),
            None => panic!(
                "{doc_rel} must quote the {circuit} fixture value at index {index} \
                 ({value}) in the same order as the fixture",
                circuit = fixture.circuit
            ),
        }
    }
}

/// Regenerate `PublicSignals` from the fixture values and check the encoded
/// order: length prefix `n`, then n x 32-byte big-endian lanes, with value i
/// in lane i. Swapping two fixture values (or two signals) fails here.
fn assert_fixture_wire_order(env: &Env, fixture: &PublicSignalFixture, pinned_order: &[&str]) {
    let mut pub_signals = Vec::new(env);
    for dec in &fixture.public_signals {
        pub_signals.push_back(fr_from_decimal(env, dec));
    }
    let signals = PublicSignals { pub_signals };

    let n = fixture.public_signals.len();
    let encoded = signals.to_bytes(env);
    assert_eq!(
        encoded.len(),
        (4 + 32 * n) as u32,
        "wire length must be 4 + 32 * {n} for {circuit}",
        circuit = fixture.circuit
    );

    let mut header = [0u8; 4];
    encoded.slice(0..4).copy_into_slice(&mut header);
    assert_eq!(
        u32::from_be_bytes(header),
        n as u32,
        "length prefix must be u32 big-endian n = {n}"
    );

    for (index, (name, dec)) in pinned_order
        .iter()
        .zip(fixture.public_signals.iter())
        .enumerate()
    {
        let start = (4 + 32 * index) as u32;
        let mut lane = [0u8; 32];
        encoded
            .slice(start..(start + 32))
            .copy_into_slice(&mut lane);
        assert_eq!(
            lane,
            decimal_to_be_bytes(dec),
            "signal index {index} (`{name}`) must land in byte lane {index}"
        );
    }

    let roundtrip = PublicSignals::from_bytes(env, &encoded).unwrap();
    assert_eq!(roundtrip.pub_signals.len(), n as u32);
    assert_eq!(roundtrip.pub_signals, signals.pub_signals);
}

#[test]
fn test_withdrawal_public_signals_from_fixture() {
    let env = Env::default();
    let fixture = parse_public_signal_fixture(WITHDRAWAL_FIXTURE_JSON);

    assert_fixture_pins_order(
        &fixture,
        &WITHDRAWAL_SIGNAL_ORDER,
        WITHDRAWAL_CIRCUIT_SRC,
        WITHDRAWAL_CIRCUIT_REL,
    );
    assert_fixture_wire_order(&env, &fixture, &WITHDRAWAL_SIGNAL_ORDER);
}

#[test]
fn test_disclosure_public_signals_from_fixture() {
    let env = Env::default();
    let fixture = parse_public_signal_fixture(DISCLOSURE_FIXTURE_JSON);

    assert_fixture_pins_order(
        &fixture,
        &DISCLOSURE_SIGNAL_ORDER,
        DISCLOSURE_CIRCUIT_SRC,
        DISCLOSURE_CIRCUIT_REL,
    );
    assert_fixture_wire_order(&env, &fixture, &DISCLOSURE_SIGNAL_ORDER);
}

// ---------------------------------------------------------------------------
// Receipt auditor-tag derivation, v2 (issue #3).
//
// auditorTag = Poseidon255(viewingKey, nullifierHash, receiptNonce). The
// per-receipt nonce is the domain separator that lets two receipts for the
// SAME withdrawal (same nullifierHash, same viewingKey) carry different
// auditorTags, so comparing tags alone no longer links repeated disclosures.
// The v1 formula was Poseidon255(viewingKey, nullifierHash): deterministic per
// (key, withdrawal), so every repeated disclosure produced an identical tag.
//
// Fixture: libs/zk/testdata/receipt_tags.json, computed with
// circuits/disclosure_witness.circom from circuits/disclosure_private.json
// sample inputs (plus a 2-arity Poseidon255 for the legacy tag). These tests
// pin the derivation format: if the circuit's tag formula, arity, or operand
// order changes, the regenerated values will no longer match. Prototype limit:
// the circuit cannot force a fresh nonce — a prover who reuses one recreates
// v1 linkability; freshness is a prover-side discipline, not a proof
// guarantee.
// ---------------------------------------------------------------------------

struct ReceiptTagFixture {
    viewing_key: String,
    nullifier_hash: String,
    legacy_auditor_tag: String,
    case_nonces: std::vec::Vec<String>,
    case_tags: std::vec::Vec<String>,
    other_viewing_key: String,
    other_nonce: String,
    other_tag: String,
}

fn parse_receipt_tag_fixture(raw: &str) -> ReceiptTagFixture {
    let json: serde_json::Value = serde_json::from_str(raw).expect("fixture is valid JSON");
    let base = json.get("base").expect("fixture must have `base`");
    let cases = json
        .get("cases")
        .and_then(|value| value.as_array())
        .expect("`cases` must be an array");
    let other = json
        .get("otherViewingKey")
        .expect("fixture must have `otherViewingKey`");

    let mut case_nonces = std::vec::Vec::new();
    let mut case_tags = std::vec::Vec::new();
    for case in cases {
        case_nonces.push(json_text_field(case, "receiptNonce"));
        case_tags.push(json_text_field(case, "auditorTag"));
    }

    ReceiptTagFixture {
        viewing_key: json_text_field(base, "viewingKey"),
        nullifier_hash: json_text_field(base, "nullifierHash"),
        legacy_auditor_tag: json_text_field(base, "legacyAuditorTag"),
        case_nonces,
        case_tags,
        other_viewing_key: json_text_field(other, "viewingKey"),
        other_nonce: json_text_field(other, "receiptNonce"),
        other_tag: json_text_field(other, "auditorTag"),
    }
}

#[test]
fn test_receipt_tag_nonce_separates_repeated_disclosures() {
    let fixture = parse_receipt_tag_fixture(RECEIPT_TAGS_FIXTURE_JSON);

    assert!(
        fixture.case_nonces.len() >= 2,
        "fixture needs at least two nonces for the same (viewingKey, nullifierHash)"
    );

    // Same viewing key + same withdrawal, different nonces -> different tags.
    // This is the property that separates repeated disclosures (issue #3).
    for (i, tag_i) in fixture.case_tags.iter().enumerate() {
        for (j, tag_j) in fixture.case_tags.iter().enumerate() {
            if i != j {
                assert_ne!(
                    tag_i,
                    tag_j,
                    "receiptNonce {a} and {b} for the same withdrawal must produce \
                     different auditorTags (reuse of a nonce would recreate v1 \
                     linkability)",
                    a = fixture.case_nonces[i],
                    b = fixture.case_nonces[j]
                );
            }
        }
    }

    // Every tag must be a canonical Fr so it lands on the wire as one 32-byte
    // lane, exactly like the public signals it ships in.
    for tag in fixture
        .case_tags
        .iter()
        .chain(std::iter::once(&fixture.legacy_auditor_tag))
    {
        let bytes = decimal_to_be_bytes(tag);
        assert!(
            is_canonical_fr_bytes_for_fixture(&bytes),
            "fixture tag {tag} must be a canonical BLS12-381 scalar"
        );
    }
}

#[test]
fn test_receipt_tag_viewing_key_and_nonce_both_change_tag() {
    let fixture = parse_receipt_tag_fixture(RECEIPT_TAGS_FIXTURE_JSON);

    // A different viewing key with a reused nonce must also produce a
    // different tag: the tag is bound to the key, not just to the nonce.
    assert_ne!(
        fixture.other_tag, fixture.case_tags[0],
        "different viewingKey with the same receiptNonce must produce a \
         different auditorTag"
    );
    for tag in &fixture.case_tags {
        assert_ne!(
            fixture.other_tag.as_str(),
            tag.as_str(),
            "other-viewing-key tag must not collide with any base-key tag"
        );
    }

    // The nonce is deliberately held constant so the tag difference is
    // attributable to the viewing key alone.
    assert_eq!(
        fixture.other_nonce, fixture.case_nonces[0],
        "otherViewingKey case must reuse the base nonce to isolate the key"
    );
}

#[test]
fn test_receipt_tag_is_stable_for_fixed_nonce_and_key() {
    let fixture = parse_receipt_tag_fixture(RECEIPT_TAGS_FIXTURE_JSON);

    // Re-deriving with the SAME nonce and key must reproduce the SAME tag —
    // otherwise an auditor could never recompute it from the receipt. The
    // pinned value is the ground truth from the circuit twin; the pairing
    // below documents the exact inputs it commits to.
    assert_eq!(
        fixture.case_nonces[0], "31337",
        "base case must stay aligned with circuits/disclosure_private.json"
    );
    assert_eq!(
        fixture.viewing_key, "99999",
        "base case must stay aligned with circuits/disclosure_private.json"
    );
    assert_eq!(
        fixture.case_tags[0],
        "48504087352648101872232104541849549896624195243212218591472176225635135984236",
        "auditorTag must remain Poseidon255(viewingKey=99999, nullifierHash, receiptNonce=31337)"
    );
}

#[test]
fn test_receipt_tag_v2_never_equals_v1_legacy_tag() {
    let fixture = parse_receipt_tag_fixture(RECEIPT_TAGS_FIXTURE_JSON);

    // The v1 tag for the same (viewingKey, nullifierHash) is pinned from the
    // 2-arity Poseidon255. It must not appear anywhere in the v2 set: v1 and
    // v2 receipts for the same withdrawal are separated by derivation format,
    // so a v2 tag can never be mistaken for (or replayed as) a v1 tag.
    assert_ne!(
        fixture.legacy_auditor_tag, fixture.case_tags[0],
        "v1 Poseidon255(viewingKey, nullifierHash) must differ from the v2 \
         nonce-derived tag for the same inputs"
    );
    for tag in &fixture.case_tags {
        assert_ne!(
            tag, &fixture.legacy_auditor_tag,
            "no v2 nonce-derived tag may equal the v1 legacy tag"
        );
    }
    assert_ne!(fixture.legacy_auditor_tag, fixture.other_tag);
}

/// Fixture values are checked against the same canonical-scalar rule the
/// on-chain `PublicSignals` decoder enforces (`libs/zk/src/lib.rs`).
fn is_canonical_fr_bytes_for_fixture(bytes: &[u8; 32]) -> bool {
    // BLS12-381 scalar modulus r (big-endian), mirroring
    // `is_canonical_fr_bytes` in the crate root.
    const R: [u8; 32] = [
        0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8,
        0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00,
        0x00, 0x01,
    ];
    for i in 0..32 {
        if bytes[i] < R[i] {
            return true;
        }
        if bytes[i] > R[i] {
            return false;
        }
    }
    false // equal to r -> non-canonical
}
