function validateSplit(input) {
    if (!input.match(/^[\d\+\/\*\-\(\)\. ]*$/)) {
        return Number.NaN;
    }
    if (input.match(/([\+\/\*\-])\1/)) {
        return Number.NaN;
    }
    try {
        v = eval(input);
    } catch (err) {
        return Number.NaN;
    }
    if (v == null) {
        return 0;
    }
    return v;
}

function calcSplit() {
    const amount = document.getElementById("amount").value;
    let total = 0;
    let values = new Array();
    const textvals = document.getElementsByClassName("share-text");
    for (i=0; i<textvals.length; i++) {
        const v = validateSplit(textvals[i].value);
        if (!isNaN(v)) {
            total += v;
        }
        values[i] = v;
    }
    for (i=0; i<textvals.length; i++) {
        const id = textvals[i].id+'-calc';
        const val = (amount*values[i]/total).toFixed(2);
        document.getElementById(id).innerHTML = val;
    }
}

window.onload=calcSplit;
